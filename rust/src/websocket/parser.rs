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

use crate::detect::uint::{detect_parse_uint, DetectUintData, DetectUintMode};
use nom7::bytes::streaming::take;
use nom7::combinator::cond;
use nom7::number::streaming::{be_u16, be_u32, be_u64, be_u8};
use nom7::IResult;
use suricata_derive::EnumStringU8;

#[derive(Clone, Debug, Default, EnumStringU8)]
#[repr(u8)]
pub enum WebSocketOpcode {
    #[default]
    Continuation = 0,
    Text = 1,
    Binary = 2,
    Ping = 8,
    Pong = 9,
    Unknown(u8),
}

#[derive(Clone, Debug, Default)]
pub struct WebSocketPdu {
    pub fin: bool,
    pub compress: bool,
    pub opcode: WebSocketOpcode,
    pub mask: Option<u32>,
    pub payload: Vec<u8>,
    pub to_skip: u64,
}

// cf rfc6455#section-5.2
pub fn parse_message(i: &[u8], max_pl_size: u64) -> IResult<&[u8], WebSocketPdu> {
    let (i, fin_op) = be_u8(i)?;
    let fin = (fin_op & 0x80) != 0;
    let compress = (fin_op & 0x40) != 0;
    let opcode = fin_op & 0xF;
    let opcode = WebSocketOpcode::from_u(opcode);
    let (i, mask_plen) = be_u8(i)?;
    let mask_flag = (mask_plen & 0x80) != 0;
    let (i, payload_len) = match mask_plen & 0x7F {
        126 => {
            let (i, val) = be_u16(i)?;
            Ok((i, val.into()))
        }
        127 => be_u64(i),
        _ => Ok((i, (mask_plen & 0x7F).into())),
    }?;
    let (i, xormask) = cond(mask_flag, take(4usize))(i)?;
    let mask = if mask_flag {
        let (_, m) = be_u32(xormask.unwrap())?;
        Some(m)
    } else {
        None
    };
    let (to_skip, payload_len) = if payload_len < max_pl_size {
        (0, payload_len)
    } else {
        (payload_len - max_pl_size, max_pl_size)
    };
    let (i, payload_raw) = take(payload_len)(i)?;
    let mut payload = payload_raw.to_vec();
    if let Some(xorkey) = xormask {
        for i in 0..payload.len() {
            payload[i] ^= xorkey[i % 4];
        }
    }
    Ok((
        i,
        WebSocketPdu {
            fin,
            compress,
            opcode,
            mask,
            payload,
            to_skip,
        },
    ))
}
