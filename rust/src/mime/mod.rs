/* Copyright (C) 2021 Open Information Security Foundation
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

use std;

use nom::combinator::rest;
use nom::error::ErrorKind;
use nom::Err;
use nom::IResult;

#[derive(Clone, Debug)]
pub struct MIMEHeaderToken<'a> {
    pub name: &'a [u8],
    pub value: &'a [u8],
}

#[derive(Clone)]
pub struct MIMEHeaderTokens<'a> {
    pub tokens: Vec<MIMEHeaderToken<'a>>,
}

pub fn mime_parse_value_delimited(input: &[u8]) -> IResult<&[u8], &[u8]> {
    let (i2, _) = tag!(input, "\"")?;
    let (i3, value) = take_until!(i2, "\"")?;
    let (i4, _) = tag!(i3, "\"")?;
    return Ok((i4, value));
}

pub fn mime_parse_header_token(input: &[u8]) -> IResult<&[u8], MIMEHeaderToken> {
    // maybe only U+0020 space and U+0009 tab
    let (i1, _) = take_while!(input, |ch: u8| ch.is_ascii_whitespace())?;
    let (i2, name) = take_until!(i1, "=")?;
    let (i3, _) = tag!(i2, "=")?;
    let (i4, value) = alt!(
        i3,
        mime_parse_value_delimited | complete!(take_until!(";")) | rest
    )?;
    let (i5, _) = opt!(i4, complete!(tag!(";")))?;
    return Ok((i5, MIMEHeaderToken { name, value }));
}

fn mime_parse_header_tokens(input: &[u8]) -> IResult<&[u8], MIMEHeaderTokens> {
    let (mut i2, _) = take_until_and_consume!(input, ";")?;
    let mut tokens = Vec::new();
    while i2.len() > 0 {
        match mime_parse_header_token(i2) {
            Ok((rem, t)) => {
                tokens.push(t);
                // should never happen
                debug_validate_bug_on!(i2.len() == rem.len());
                if i2.len() == rem.len() {
                    //infinite loop
                    return Err(Err::Error((input, ErrorKind::Eof)));
                }
                i2 = rem;
            }
            Err(_) => {
                // keep first tokens is error in remaining buffer
                break;
            }
        }
    }
    return Ok((i2, MIMEHeaderTokens { tokens }));
}

fn mime_find_header_token<'a>(
    header: &'a [u8], token: &[u8], sections_values: &'a mut Vec<u8>,
) -> Result<&'a [u8], ()> {
    match mime_parse_header_tokens(header) {
        Ok((_rem, t)) => {
            // in case of multiple sections for the parameter cf RFC2231
            let mut current_section_seen = 0;
            let mut current_section_slice = Vec::new();

            // look for the specific token
            for i in 0..t.tokens.len() {
                if t.tokens[i].name == token {
                    // easy nominal case
                    return Ok(t.tokens[i].value);
                }

                // check for initial section of a parameter
                if current_section_seen == 0
                    && t.tokens[i].name.len() == token.len() + 2
                    && t.tokens[i].name[t.tokens[i].name.len() - 2] == b'*'
                    && t.tokens[i].name[t.tokens[i].name.len() - 1] == b'0'
                    && &t.tokens[i].name[..t.tokens[i].name.len() - 2] == token
                {
                    // initial section found, get name of next section in current_section_slice
                    current_section_seen = 1;
                    sections_values.extend_from_slice(t.tokens[i].value);
                    current_section_slice.extend_from_slice(t.tokens[i].name);
                    current_section_slice[t.tokens[i].name.len() - 1] = b'1';
                } else if current_section_seen > 0 {
                    if t.tokens[i].name == current_section_slice {
                        current_section_seen += 1;
                        sections_values.extend_from_slice(t.tokens[i].value);
                        let nbdigits = current_section_slice.len() - token.len() - 1;
                        current_section_slice.truncate(current_section_slice.len() - nbdigits);
                        current_section_slice
                            .extend_from_slice(current_section_seen.to_string().as_bytes());
                    }
                }
            }
            if current_section_seen > 0 {
                loop {
                    // we have at least the initial section
                    // try looping until we do not find anymore a next section
                    let mut increased = false;
                    for i in 0..t.tokens.len() {
                        if t.tokens[i].name == current_section_slice {
                            current_section_seen += 1;
                            sections_values.extend_from_slice(t.tokens[i].value);
                            let nbdigits = current_section_slice.len() - token.len() - 1;
                            current_section_slice.truncate(current_section_slice.len() - nbdigits);
                            current_section_slice
                                .extend_from_slice(current_section_seen.to_string().as_bytes());
                            increased = true;
                        }
                    }
                    if !increased {
                        break;
                    }
                }
                return Ok(sections_values);
            }
        }
        Err(_) => {
            return Err(());
        }
    }
    return Err(());
}

// TODO ? export with "constants" in cbindgen
// and use in outbuf definition for rs_mime_find_header_token
// but other constants are now defined twice in rust and in C
const RS_MIME_MAX_TOKEN_LEN: usize = 255;

#[no_mangle]
pub unsafe extern "C" fn rs_mime_find_header_token(
    hinput: *const u8, hlen: u32, tinput: *const u8, tlen: u32, outbuf: &mut [u8; 255],
    outlen: *mut u32,
) -> bool {
    let hbuf = std::slice::from_raw_parts(hinput, hlen as usize);
    let tbuf = std::slice::from_raw_parts(tinput, tlen as usize);
    let mut sections_values = Vec::new();
    match mime_find_header_token(hbuf, tbuf, &mut sections_values) {
        Ok(value) => {
            // limit the copy to the supplied buffer size
            if value.len() <= RS_MIME_MAX_TOKEN_LEN {
                outbuf[..value.len()].clone_from_slice(value);
            } else {
                outbuf.clone_from_slice(&value[..RS_MIME_MAX_TOKEN_LEN]);
            }
            *outlen = value.len() as u32;
            return true;
        }
        _ => {}
    }
    return false;
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_mime_find_header_token() {
        let mut outvec = Vec::new();
        let undelimok = mime_find_header_token(
            "attachment; filename=test;".as_bytes(),
            "filename".as_bytes(),
            &mut outvec,
        );
        assert_eq!(undelimok, Ok("test".as_bytes()));

        let delimok = mime_find_header_token(
            "attachment; filename=\"test2\";".as_bytes(),
            "filename".as_bytes(),
            &mut outvec,
        );
        assert_eq!(delimok, Ok("test2".as_bytes()));

        let evasion_othertoken = mime_find_header_token(
            "attachment; dummy=\"filename=wrong\"; filename=real;".as_bytes(),
            "filename".as_bytes(),
            &mut outvec,
        );
        assert_eq!(evasion_othertoken, Ok("real".as_bytes()));

        let evasion_suffixtoken = mime_find_header_token(
            "attachment; notafilename=wrong; filename=good;".as_bytes(),
            "filename".as_bytes(),
            &mut outvec,
        );
        assert_eq!(evasion_suffixtoken, Ok("good".as_bytes()));

        let badending = mime_find_header_token(
            "attachment; filename=oksofar; badending".as_bytes(),
            "filename".as_bytes(),
            &mut outvec,
        );
        assert_eq!(badending, Ok("oksofar".as_bytes()));

        let missend = mime_find_header_token(
            "attachment; filename=test".as_bytes(),
            "filename".as_bytes(),
            &mut outvec,
        );
        assert_eq!(missend, Ok("test".as_bytes()));

        assert_eq!(outvec.len(), 0);
        let multi = mime_find_header_token(
            "attachment; filename*0=abc; filename*1=\"def\";".as_bytes(),
            "filename".as_bytes(),
            &mut outvec,
        );
        assert_eq!(multi, Ok("abcdef".as_bytes()));
        outvec.clear();

        let multi = mime_find_header_token(
            "attachment; filename*1=456; filename*0=\"123\"".as_bytes(),
            "filename".as_bytes(),
            &mut outvec,
        );
        assert_eq!(multi, Ok("123456".as_bytes()));
        outvec.clear();
    }
}
