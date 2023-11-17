/* Copyright (C) 2023 Open Information Security Foundation
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

use nom7::bytes::streaming::take;
use nom7::error::{make_error, ErrorKind};
use nom7::multi::count;
use nom7::number::streaming::{le_u16, le_u32, le_u64, le_u8};
use nom7::IResult;

pub const ENIP_STATUS_SUCCESS: u32 = 0;
pub const ENIP_STATUS_INVALID_CMD: u32 = 1;
pub const ENIP_STATUS_NO_RESOURCES: u32 = 2;
pub const ENIP_STATUS_INCORRECT_DATA: u32 = 3;
pub const ENIP_STATUS_INVALID_SESSION: u32 = 0x64;
pub const ENIP_STATUS_INVALID_LENGTH: u32 = 0x65;
pub const ENIP_STATUS_UNSUPPORTED_PROT_REV: u32 = 0x69;
//Found in wireshark
pub const ENIP_STATUS_ENCAP_HEADER_ERROR: u32 = 0x6A;

pub fn enip_status_string(v: u32) -> Option<&'static str> {
    match v {
        ENIP_STATUS_SUCCESS => Some("Success"),
        ENIP_STATUS_INVALID_CMD => Some("InvalidCmd"),
        ENIP_STATUS_NO_RESOURCES => Some("NoResources"),
        ENIP_STATUS_INCORRECT_DATA => Some("IncorrectData"),
        ENIP_STATUS_INVALID_SESSION => Some("InvalidSession"),
        ENIP_STATUS_INVALID_LENGTH => Some("InvalidLength"),
        ENIP_STATUS_UNSUPPORTED_PROT_REV => Some("UnsupportedProtRev"),
        ENIP_STATUS_ENCAP_HEADER_ERROR => Some("EncapHeaderError"),
        _ => None,
    }
}

pub const ENIP_CMD_NOP: u16 = 0;
pub const ENIP_CMD_LIST_SERVICES: u16 = 4;
pub const ENIP_CMD_LIST_IDENTITY: u16 = 0x63;
pub const ENIP_CMD_LIST_INTERFACES: u16 = 0x64;
pub const ENIP_CMD_REGISTER_SESSION: u16 = 0x65;
pub const ENIP_CMD_UNREGISTER_SESSION: u16 = 0x66;
pub const ENIP_CMD_SEND_RRDATA: u16 = 0x6F;
pub const ENIP_CMD_SEND_UNIT_DATA: u16 = 0x70;
pub const ENIP_CMD_INDICATE_STATUS: u16 = 0x72;
pub const ENIP_CMD_CANCEL: u16 = 0x73;

pub fn enip_command_string(v: u16) -> Option<&'static str> {
    match v {
        ENIP_CMD_NOP => Some("Nop"),
        ENIP_CMD_LIST_SERVICES => Some("ListServices"),
        ENIP_CMD_LIST_IDENTITY => Some("ListIdentity"),
        ENIP_CMD_LIST_INTERFACES => Some("ListInterfaces"),
        ENIP_CMD_REGISTER_SESSION => Some("RegisterSession"),
        ENIP_CMD_UNREGISTER_SESSION => Some("UnregisterSession"),
        ENIP_CMD_SEND_RRDATA => Some("SendRRData"),
        ENIP_CMD_SEND_UNIT_DATA => Some("SendUnitData"),
        ENIP_CMD_INDICATE_STATUS => Some("IndicateStatus"),
        ENIP_CMD_CANCEL => Some("Cancel"),
        _ => None,
    }
}

#[derive(Clone, Debug, Default)]
pub struct EnipHeader {
    pub cmd: u16,
    pub pdulen: u16,
    pub session: u32,
    pub status: u32,
    pub context: u64,
    pub options: u32,
}

pub fn parse_enip_header(i: &[u8]) -> IResult<&[u8], EnipHeader> {
    let (i, cmd) = le_u16(i)?;
    let (i, pdulen) = le_u16(i)?;
    let (i, session) = le_u32(i)?;
    let (i, status) = le_u32(i)?;
    let (i, context) = le_u64(i)?;
    let (i, options) = le_u32(i)?;
    Ok((
        i,
        EnipHeader {
            cmd,
            pdulen,
            session,
            status,
            context,
            options,
        },
    ))
}

pub fn parse_enip_list_interfaces(i: &[u8]) -> IResult<&[u8], Vec<u16>> {
    let (i, nb) = le_u16(i)?;
    let (i, r) = count(le_u16, nb.into())(i)?;
    Ok((i, r))
}

#[derive(Clone, Debug, Default)]
pub enum EnipPayload {
    #[default]
    Unparsed,
    CIP(EnipCIP),
}

#[derive(Clone, Debug, Default)]
pub struct EnipItemConnBased {
    pub conn_id: u32,
}

#[derive(Clone, Debug, Default)]
pub struct EnipItemSequenceAddr {
    pub conn_id: u32,
    pub seq_num: u32,
}

#[derive(Clone, Debug, Default)]
pub struct EnipCipPathSegment {
    pub segment_type: u8,
    pub value: u16,
}

#[derive(Clone, Debug, Default)]
pub struct EnipCipRequest {
    pub path: Vec<EnipCipPathSegment>,
}

#[derive(Clone, Debug, Default)]
pub struct EnipCipResponse {}

#[derive(Clone, Debug, Default)]
pub enum CipDir {
    #[default]
    None,
    Request(EnipCipRequest),
    Response(EnipCipResponse),
}

#[derive(Clone, Debug, Default)]
pub struct CipData {
    pub service: u8,
    pub cipdir: CipDir,
}

pub const ENIP_CIP_PATH_CLASS_8BIT: u8 = 0x20;
pub const ENIP_CIP_PATH_INSTANCE_8BIT: u8 = 0x24;
pub const ENIP_CIP_PATH_ATTR_8BIT: u8 = 0x30;
pub const ENIP_CIP_PATH_CLASS_16BIT: u8 = 0x21;
pub const ENIP_CIP_PATH_INSTANCE_16BIT: u8 = 0x25;
pub const ENIP_CIP_PATH_ATTR_16BIT: u8 = 0x31;

pub fn parse_cip_path_segment(i: &[u8]) -> IResult<&[u8], EnipCipPathSegment> {
    let (i, segment_type) = le_u8(i)?;
    let (i, value) = match segment_type {
        ENIP_CIP_PATH_CLASS_8BIT | ENIP_CIP_PATH_INSTANCE_8BIT | ENIP_CIP_PATH_ATTR_8BIT => {
            let (i, v) = le_u8(i)?;
            Ok((i, v as u16))
        }
        ENIP_CIP_PATH_CLASS_16BIT | ENIP_CIP_PATH_INSTANCE_16BIT | ENIP_CIP_PATH_ATTR_16BIT => {
            le_u16(i)
        }
        // There may be more cases to handle
        _ => Err(nom7::Err::Error(make_error(i, ErrorKind::Switch))),
    }?;
    return Ok((
        i,
        EnipCipPathSegment {
            segment_type,
            value,
        },
    ));
}

pub fn parse_cip_path(i: &[u8]) -> IResult<&[u8], Vec<EnipCipPathSegment>> {
    let (i, nb) = le_u8(i)?;
    let (i, data) = take(2 * (nb as usize))(i)?;
    let mut rem = data;
    let mut segments = Vec::new();
    while rem.len() > 0 {
        let (rem2, seg) = parse_cip_path_segment(rem)?;
        segments.push(seg);
        rem = rem2;
    }
    return Ok((i, segments));
}

pub fn parse_cip_request(i: &[u8]) -> IResult<&[u8], EnipCipRequest> {
    let (i, path) = parse_cip_path(i)?;
    //TODOlol more based on service -> attribute list, multi service packet
    return Ok((i, EnipCipRequest { path }));
}

pub fn parse_cip_response(i: &[u8]) -> IResult<&[u8], EnipCipResponse> {
    //TODOlol more
    return Ok((i, EnipCipResponse {}));
}

pub fn parse_cip(i: &[u8]) -> IResult<&[u8], CipData> {
    let (i, service) = le_u8(i)?;
    let (i, cipdir) = if service & 0x80 == 0 {
        let (i, req) = parse_cip_request(i)?;
        Ok((i, CipDir::Request(req)))
    } else {
        let (i, resp) = parse_cip_response(i)?;
        Ok((i, CipDir::Response(resp)))
    }?;
    return Ok((
        i,
        CipData {
            service: service & 0x7F,
            cipdir,
        },
    ));
}

#[derive(Clone, Debug, Default)]
pub struct EnipItemData {
    pub seq_num: Option<u16>,
    pub cip: CipData,
}

#[derive(Clone, Debug, Default)]
pub enum EnipItemPayload {
    #[default]
    Unparsed,
    ConnBased(EnipItemConnBased),
    SequenceAddr(EnipItemSequenceAddr),
    Data(EnipItemData),
}

#[derive(Clone, Debug, Default)]
pub struct EnipCipItem {
    pub item_type: u16,
    pub payload: EnipItemPayload,
}

pub const ENIP_ITEM_TYPE_CONNECTION_BASED: u16 = 0xa1;
pub const ENIP_ITEM_TYPE_SEQUENCE_ADDR: u16 = 0x8002;
pub const ENIP_ITEM_TYPE_CONNECTED_DATA: u16 = 0xb1;
pub const ENIP_ITEM_TYPE_UNCONNECTED_DATA: u16 = 0xb2;

pub fn parse_enip_cip_item(i: &[u8]) -> IResult<&[u8], EnipCipItem> {
    let (i, item_type) = le_u16(i)?;
    let (i, item_length) = le_u16(i)?;
    let (i, data) = take(item_length as usize)(i)?;
    let (_, payload) = match item_type {
        ENIP_ITEM_TYPE_CONNECTION_BASED => {
            let (data, conn_id) = le_u32(data)?;
            Ok((
                data,
                EnipItemPayload::ConnBased(EnipItemConnBased { conn_id }),
            ))
        }
        ENIP_ITEM_TYPE_SEQUENCE_ADDR => {
            let (data, conn_id) = le_u32(data)?;
            let (data, seq_num) = le_u32(data)?;
            Ok((
                data,
                EnipItemPayload::SequenceAddr(EnipItemSequenceAddr { conn_id, seq_num }),
            ))
        }
        ENIP_ITEM_TYPE_CONNECTED_DATA => {
            let (data, seq_num) = le_u16(data)?;
            let (_, cip) = parse_cip(data)?;
            Ok((
                data,
                EnipItemPayload::Data(EnipItemData {
                    seq_num: Some(seq_num),
                    cip,
                }),
            ))
        }
        ENIP_ITEM_TYPE_UNCONNECTED_DATA => {
            let (_, cip) = parse_cip(data)?;
            Ok((
                data,
                EnipItemPayload::Data(EnipItemData { seq_num: None, cip }),
            ))
        }
        _ => Ok((data, EnipItemPayload::Unparsed)),
    }?;
    Ok((i, EnipCipItem { item_type, payload }))
}

#[derive(Clone, Debug, Default)]
pub struct EnipCIP {
    pub handle: u32,
    pub timeout: u16,
    pub items: Vec<EnipCipItem>,
}

pub fn parse_enip_cip(i: &[u8]) -> IResult<&[u8], EnipCIP> {
    let (i, handle) = le_u32(i)?;
    let (i, timeout) = le_u16(i)?;
    let (i, nb) = le_u16(i)?;
    let (i, items) = count(parse_enip_cip_item, nb.into())(i)?;
    Ok((
        i,
        EnipCIP {
            handle,
            timeout,
            items,
        },
    ))
}

#[derive(Clone, Debug, Default)]
pub struct EnipPdu {
    pub header: EnipHeader,
    pub payload: EnipPayload,
}

pub fn parse_enip_pdu(i: &[u8]) -> IResult<&[u8], EnipPdu> {
    let (i, header) = parse_enip_header(i)?;
    let (i, data) = take(header.pdulen as usize)(i)?;
    match header.cmd {
        ENIP_CMD_SEND_RRDATA | ENIP_CMD_SEND_UNIT_DATA => {
            if let Ok((_, cip)) = parse_enip_cip(data) {
                return Ok((
                    i,
                    EnipPdu {
                        header,
                        payload: EnipPayload::CIP(cip),
                    },
                ));
            } //TODOlol else set event
        }
        //TODOlol listid
        _ => {}
    }
    Ok((
        i,
        EnipPdu {
            header,
            payload: EnipPayload::Unparsed,
        },
    ))
}
