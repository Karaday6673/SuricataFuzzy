/* Copyright (C) 2017-2018 Open Information Security Foundation
 *
 * You can copy, redistribute or modify this Program under the terms of
 * the GNU General Public License version 2 as published by the Free
 * Software Foundation.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * version 2 along with this program; if not, write to the Free Software
 * Foundation, Inc., 51 Franklin Street, Fifth Floor, Boston, MA
 * 02110-1301, USA.
 */

// written by Pierre Chifflier  <chifflier@wzdftpd.net>

use snmp::snmp_parser::*;
use core;
use core::{AppProto,Flow,ALPROTO_UNKNOWN,ALPROTO_FAILED,STREAM_TOSERVER,STREAM_TOCLIENT};
use applayer;
use parser::*;
use libc;
use std;
use std::ffi::{CStr,CString};
use std::mem::transmute;

use log::*;

use der_parser::{DerObjectContent,parse_der_sequence};
use der_parser::oid::Oid;
use nom::{ErrorKind,IResult};

#[repr(u32)]
pub enum SNMPEvent {
    MalformedData = 0,
    UnknownSecurityModel,
}

pub struct SNMPState {
    /// SNMP protocol version
    pub version: u32,

    /// List of transactions for this session
    transactions: Vec<SNMPTransaction>,

    /// tx counter for assigning incrementing id's to tx's
    tx_id: u64,
}

pub struct SNMPPduInfo {
    pub pdu_type: PduType,

    pub err: ErrorStatus,

    pub trap_type: Option<(TrapType,Oid,NetworkAddress)>,

    pub vars: Vec<Oid>,
}

pub struct SNMPTransaction {
    /// PDU info, if present (and cleartext)
    pub info: Option<SNMPPduInfo>,

    /// Community, if present (SNMPv2)
    pub community: Option<String>,

    /// USM info, if present (SNMPv3)
    pub usm: Option<String>,

    /// True if transaction was encrypted
    pub encrypted: bool,

    /// The internal transaction id
    id: u64,

    /// The detection engine state, if present
    de_state: Option<*mut core::DetectEngineState>,

    /// The events associated with this transaction
    events: *mut core::AppLayerDecoderEvents,

    logged: applayer::LoggerFlags,
}



impl SNMPState {
    pub fn new() -> SNMPState {
        SNMPState{
            version: 0,
            transactions: Vec::new(),
            tx_id: 0,
        }
    }
}

impl Default for SNMPPduInfo {
    fn default() -> SNMPPduInfo {
        SNMPPduInfo{
            pdu_type: PduType(0),
            err: ErrorStatus::NoError,
            trap_type: None,
            vars: Vec::new()
        }
    }
}

impl SNMPState {
    fn add_pdu_info(&mut self, pdu: &SnmpPdu, tx: &mut SNMPTransaction) {
        let mut pdu_info = SNMPPduInfo::default();
        pdu_info.pdu_type = pdu.pdu_type();
        match pdu {
            SnmpPdu::Generic(ref pdu) => {
                pdu_info.err = pdu.err;
            },
            SnmpPdu::TrapV1(ref t)    => {
                pdu_info.trap_type = Some((t.generic_trap,t.enterprise.clone(),t.agent_addr.clone()));
            }
        }
        for ref var in pdu.vars_iter() {
            pdu_info.vars.push(var.oid.clone());
        }
        tx.info = Some(pdu_info);
    }

    fn parse_v1_2(&mut self, i: &[u8], _direction: u8) -> i32 {
        match parse_snmp_v1(i) {
            IResult::Done(_rem,r) => {
                let mut tx = self.new_tx();
                self.add_pdu_info(&r.pdu, &mut tx);
                tx.community = Some(r.community.clone());
                self.transactions.push(tx);
                0
            },
            _e => {
                SCLogInfo!("parse_snmp_v1 failed: {:?}", _e);
                self.set_event(SNMPEvent::MalformedData);
                -1
            },
        }
    }

    fn parse_v3(&mut self, i: &[u8], _direction: u8) -> i32 {
        match parse_snmp_v3(i) {
            IResult::Done(_rem,r) => {
                let mut tx = self.new_tx();
                match r.data {
                    ScopedPduData::Plaintext(pdu) => {
                        self.add_pdu_info(&pdu.data, &mut tx);
                    },
                    _                             => {
                        tx.encrypted = true;
                    }
                }
                match r.security_params {
                    SecurityParameters::USM(usm) => {
                        tx.usm = Some(usm.msg_user_name.clone());
                    },
                    _                            => {
                        self.set_event_tx(&mut tx, SNMPEvent::UnknownSecurityModel);
                    }
                }
                self.transactions.push(tx);
                0
            },
            _e => {
                SCLogInfo!("parse_snmp_v3 failed: {:?}", _e);
                self.set_event(SNMPEvent::MalformedData);
                -1
            },
        }
    }

    /// Parse an SNMP request message
    ///
    /// Returns The number of messages parsed, or -1 on error
    fn parse(&mut self, i: &[u8], direction: u8) -> i32 {
        if self.version == 0 {
            match parse_pdu_enveloppe_version(i) {
                IResult::Done(_,x) => self.version = x,
                _                  => (),
            }
        }
        match self.version {
            1 | 2 => self.parse_v1_2(i, direction),
            3     => self.parse_v3(i, direction),
            _     => -1,
        }
    }

    fn free(&mut self) {
        // All transactions are freed when the `transactions` object is freed.
        // But let's be explicit
        self.transactions.clear();
    }

    fn new_tx(&mut self) -> SNMPTransaction {
        self.tx_id += 1;
        SNMPTransaction::new(self.tx_id)
    }

    fn get_tx_by_id(&mut self, tx_id: u64) -> Option<&SNMPTransaction> {
        self.transactions.iter().rev().find(|&tx| tx.id == tx_id + 1)
    }

    fn free_tx(&mut self, tx_id: u64) {
        let tx = self.transactions.iter().position(|ref tx| tx.id == tx_id + 1);
        debug_assert!(tx != None);
        if let Some(idx) = tx {
            let _ = self.transactions.remove(idx);
        }
    }

    /// Set an event. The event is set on the most recent transaction.
    fn set_event(&mut self, event: SNMPEvent) {
        if let Some(tx) = self.transactions.last_mut() {
            let ev = event as u8;
            core::sc_app_layer_decoder_events_set_event_raw(&mut tx.events, ev);
        }
    }

    /// Set an event on a specific transaction.
    fn set_event_tx(&self, tx: &mut SNMPTransaction, event: SNMPEvent) {
        core::sc_app_layer_decoder_events_set_event_raw(&mut tx.events, event as u8);
    }

    // for use with the C API call StateGetTxIterator
    pub fn get_tx_iterator(&mut self, min_tx_id: u64, state: &mut u64) ->
        Option<(&SNMPTransaction, u64, bool)>
    {
        let mut index = *state as usize;
        let len = self.transactions.len();

        // find tx that is >= min_tx_id
        while index < len {
            let tx = &self.transactions[index];
            if tx.id < min_tx_id + 1 {
                index += 1;
                continue;
            }
            *state = index as u64 + 1;
            //SCLogDebug!("returning tx_id {} has_next? {} (len {} index {}), tx {:?}",
            //        tx.id - 1, (len - index) > 1, len, index, tx);
            return Some((tx, tx.id - 1, (len - index) > 1));
        }
        return None;
    }
}

impl SNMPTransaction {
    pub fn new(id: u64) -> SNMPTransaction {
        SNMPTransaction {
            info: None,
            community: None,
            usm: None,
            encrypted: false,
            id: id,
            de_state: None,
            events: std::ptr::null_mut(),
            logged: applayer::LoggerFlags::new(),
        }
    }

    fn free(&mut self) {
        if self.events != std::ptr::null_mut() {
            core::sc_app_layer_decoder_events_free_events(&mut self.events);
        }
    }
}

impl Drop for SNMPTransaction {
    fn drop(&mut self) {
        self.free();
    }
}






/// Returns *mut SNMPState
#[no_mangle]
pub extern "C" fn rs_snmp_state_new() -> *mut libc::c_void {
    let state = SNMPState::new();
    let boxed = Box::new(state);
    return unsafe{std::mem::transmute(boxed)};
}

/// Params:
/// - state: *mut SNMPState as void pointer
#[no_mangle]
pub extern "C" fn rs_snmp_state_free(state: *mut libc::c_void) {
    // Just unbox...
    let mut snmp_state: Box<SNMPState> = unsafe{std::mem::transmute(state)};
    snmp_state.free();
}

#[no_mangle]
pub extern "C" fn rs_snmp_parse_request(_flow: *const core::Flow,
                                       state: *mut libc::c_void,
                                       _pstate: *mut libc::c_void,
                                       input: *const libc::uint8_t,
                                       input_len: u32,
                                       _data: *const libc::c_void,
                                       _flags: u8) -> i32 {
    let buf = build_slice!(input,input_len as usize);
    let state = cast_pointer!(state,SNMPState);
    state.parse(buf, STREAM_TOSERVER)
}

#[no_mangle]
pub extern "C" fn rs_snmp_parse_response(_flow: *const core::Flow,
                                       state: *mut libc::c_void,
                                       _pstate: *mut libc::c_void,
                                       input: *const libc::uint8_t,
                                       input_len: u32,
                                       _data: *const libc::c_void,
                                       _flags: u8) -> i32 {
    let buf = build_slice!(input,input_len as usize);
    let state = cast_pointer!(state,SNMPState);
    state.parse(buf, STREAM_TOCLIENT)
}

#[no_mangle]
pub extern "C" fn rs_snmp_state_get_tx(state: *mut libc::c_void,
                                      tx_id: libc::uint64_t)
                                      -> *mut libc::c_void
{
    let state = cast_pointer!(state,SNMPState);
    match state.get_tx_by_id(tx_id) {
        Some(tx) => unsafe{std::mem::transmute(tx)},
        None     => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn rs_snmp_state_get_tx_count(state: *mut libc::c_void)
                                            -> libc::uint64_t
{
    let state = cast_pointer!(state,SNMPState);
    state.tx_id
}

#[no_mangle]
pub extern "C" fn rs_snmp_state_tx_free(state: *mut libc::c_void,
                                       tx_id: libc::uint64_t)
{
    let state = cast_pointer!(state,SNMPState);
    state.free_tx(tx_id);
}

#[no_mangle]
pub extern "C" fn rs_snmp_state_progress_completion_status(
    _direction: libc::uint8_t)
    -> libc::c_int
{
    return 1;
}

#[no_mangle]
pub extern "C" fn rs_snmp_tx_get_alstate_progress(_tx: *mut libc::c_void,
                                                 _direction: libc::uint8_t)
                                                 -> libc::c_int
{
    1
}





#[no_mangle]
pub extern "C" fn rs_snmp_tx_set_logged(_state: *mut libc::c_void,
                                       tx: *mut libc::c_void,
                                       logged: libc::uint32_t)
{
    let tx = cast_pointer!(tx,SNMPTransaction);
    tx.logged.set(logged);
}

#[no_mangle]
pub extern "C" fn rs_snmp_tx_get_logged(_state: *mut libc::c_void,
                                       tx: *mut libc::c_void)
                                       -> u32
{
    let tx = cast_pointer!(tx,SNMPTransaction);
    return tx.logged.get();
}


#[no_mangle]
pub extern "C" fn rs_snmp_state_set_tx_detect_state(
    tx: *mut libc::c_void,
    de_state: &mut core::DetectEngineState) -> libc::c_int
{
    let tx = cast_pointer!(tx,SNMPTransaction);
    tx.de_state = Some(de_state);
    0
}

#[no_mangle]
pub extern "C" fn rs_snmp_state_get_tx_detect_state(
    tx: *mut libc::c_void)
    -> *mut core::DetectEngineState
{
    let tx = cast_pointer!(tx,SNMPTransaction);
    match tx.de_state {
        Some(ds) => ds,
        None => std::ptr::null_mut(),
    }
}


#[no_mangle]
pub extern "C" fn rs_snmp_state_get_events(state: *mut libc::c_void,
                                          tx_id: libc::uint64_t)
                                          -> *mut core::AppLayerDecoderEvents
{
    let state = cast_pointer!(state,SNMPState);
    match state.get_tx_by_id(tx_id) {
        Some(tx) => tx.events,
        _        => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn rs_snmp_state_get_event_info(event_name: *const libc::c_char,
                                              event_id: *mut libc::c_int,
                                              event_type: *mut core::AppLayerEventType)
                                              -> libc::c_int
{
    if event_name == std::ptr::null() { return -1; }
    let c_event_name: &CStr = unsafe { CStr::from_ptr(event_name) };
    let event = match c_event_name.to_str() {
        Ok(s) => {
            match s {
                "malformed_data"         => SNMPEvent::MalformedData as i32,
                "unknown_security_model" => SNMPEvent::UnknownSecurityModel as i32,
                _                        => -1, // unknown event
            }
        },
        Err(_) => -1, // UTF-8 conversion failed
    };
    unsafe{
        *event_type = core::APP_LAYER_EVENT_TYPE_TRANSACTION;
        *event_id = event as libc::c_int;
    };
    0
}

// for use with the C API call StateGetTxIterator
#[no_mangle]
pub extern "C" fn rs_snmp_state_get_tx_iterator(
                                      state: &mut SNMPState,
                                      min_tx_id: libc::uint64_t,
                                      istate: &mut libc::uint64_t)
                                      -> applayer::AppLayerGetTxIterTuple
{
    match state.get_tx_iterator(min_tx_id, istate) {
        Some((tx, out_tx_id, has_next)) => {
            let c_tx = unsafe { transmute(tx) };
            let ires = applayer::AppLayerGetTxIterTuple::with_values(c_tx, out_tx_id, has_next);
            return ires;
        }
        None => {
            return applayer::AppLayerGetTxIterTuple::not_found();
        }
    }
}

// for use with the C API call StateGetTxIterator
#[no_mangle]
pub extern "C" fn rs_snmp_get_tx_iterator(_ipproto: u8,
                                          _alproto: AppProto,
                                          alstate: *mut libc::c_void,
                                          min_tx_id: u64,
                                          _max_tx_id: u64,
                                          istate: &mut u64) -> applayer::AppLayerGetTxIterTuple
{
    let state = cast_pointer!(alstate,SNMPState);
    match state.get_tx_iterator(min_tx_id, istate) {
        Some((tx, out_tx_id, has_next)) => {
            let c_tx = unsafe { transmute(tx) };
            let ires = applayer::AppLayerGetTxIterTuple::with_values(c_tx, out_tx_id, has_next);
            return ires;
        }
        None => {
            return applayer::AppLayerGetTxIterTuple::not_found();
        }
    }
}



static mut ALPROTO_SNMP : AppProto = ALPROTO_UNKNOWN;

// Read PDU sequence and extract version, if similar to SNMP definition
fn parse_pdu_enveloppe_version(i:&[u8]) -> IResult<&[u8],u32> {
    match parse_der_sequence(i) {
        IResult::Done(_,x)     => {
            match x.content {
                DerObjectContent::Sequence(ref v) => {
                    if v.len() == 3 {
                        match v[0].as_u32()  {
                            Ok(0) => { return IResult::Done(i,1); }, // possibly SNMPv1
                            Ok(1) => { return IResult::Done(i,2); }, // possibly SNMPv2c
                            _     => ()
                        }
                    } else if v.len() == 4 && v[0].as_u32() == Ok(3) {
                        return IResult::Done(i,3); // possibly SNMPv3
                    }
                },
                _ => ()
            };
            IResult::Error(error_position!(ErrorKind::Verify,i))
        },
        IResult::Incomplete(i) => IResult::Incomplete(i),
        IResult::Error(_)      => IResult::Error(error_position!(ErrorKind::Verify,i))
    }
}

#[no_mangle]
pub extern "C" fn rs_snmp_probing_parser(_flow: *const Flow, input:*const libc::uint8_t, input_len: u32) -> AppProto {
    let slice = build_slice!(input,input_len as usize);
    let alproto = unsafe{ ALPROTO_SNMP };
    if slice.len() < 4 { return unsafe{ALPROTO_FAILED}; }
    match parse_pdu_enveloppe_version(slice) {
        IResult::Done(_,_)     => alproto,
        IResult::Incomplete(_) => ALPROTO_UNKNOWN,
        _                      => unsafe{ALPROTO_FAILED},
    }
}

const PARSER_NAME : &'static [u8] = b"snmp\0";

#[no_mangle]
pub unsafe extern "C" fn rs_register_snmp_parser() {
    let default_port = CString::new("161").unwrap();
    let mut parser = RustParser {
        name              : PARSER_NAME.as_ptr() as *const libc::c_char,
        default_port      : default_port.as_ptr(),
        ipproto           : libc::IPPROTO_UDP,
        probe_ts          : rs_snmp_probing_parser,
        probe_tc          : rs_snmp_probing_parser,
        min_depth         : 0,
        max_depth         : 16,
        state_new         : rs_snmp_state_new,
        state_free        : rs_snmp_state_free,
        tx_free           : rs_snmp_state_tx_free,
        parse_ts          : rs_snmp_parse_request,
        parse_tc          : rs_snmp_parse_response,
        get_tx_count      : rs_snmp_state_get_tx_count,
        get_tx            : rs_snmp_state_get_tx,
        tx_get_comp_st    : rs_snmp_state_progress_completion_status,
        tx_get_progress   : rs_snmp_tx_get_alstate_progress,
        get_tx_logged     : Some(rs_snmp_tx_get_logged),
        set_tx_logged     : Some(rs_snmp_tx_set_logged),
        get_de_state      : rs_snmp_state_get_tx_detect_state,
        set_de_state      : rs_snmp_state_set_tx_detect_state,
        get_events        : Some(rs_snmp_state_get_events),
        get_eventinfo     : Some(rs_snmp_state_get_event_info),
        localstorage_new  : None,
        localstorage_free : None,
        get_tx_mpm_id     : None,
        set_tx_mpm_id     : None,
        get_files         : None,
        get_tx_iterator   : None,
    };
    let ip_proto_str = CString::new("udp").unwrap();
    if AppLayerProtoDetectConfProtoDetectionEnabled(ip_proto_str.as_ptr(), parser.name) != 0 {
        // port 161
        let alproto = AppLayerRegisterProtocolDetection(&parser, 1);
        // store the allocated ID for the probe function
        ALPROTO_SNMP = alproto;
        if AppLayerParserConfParserEnabled(ip_proto_str.as_ptr(), parser.name) != 0 {
            let _ = AppLayerRegisterParser(&parser, alproto);
        }
        AppLayerParserRegisterGetTxIterator(libc::IPPROTO_UDP as u8, alproto, rs_snmp_get_tx_iterator);
        // port 162
        let default_port_traps = CString::new("162").unwrap();
        parser.default_port = default_port_traps.as_ptr();
        let _ = AppLayerRegisterProtocolDetection(&parser, 1);
        if AppLayerParserConfParserEnabled(ip_proto_str.as_ptr(), parser.name) != 0 {
            let _ = AppLayerRegisterParser(&parser, alproto);
        }
    } else {
        SCLogDebug!("Protocol detecter and parser disabled for SNMP.");
    }
}
