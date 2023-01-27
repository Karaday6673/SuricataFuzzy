/* Copyright (C) 2020 Open Information Security Foundation
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

use std::ffi::CStr;
use std::os::raw::c_char;

use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

#[no_mangle]
pub unsafe extern "C" fn rs_check_utf8(val: *const c_char) -> bool {
    CStr::from_ptr(val).to_str().is_ok()
}

#[no_mangle]
pub unsafe extern "C" fn rs_tls_create_utc_iso_time_string(
    nsecs: i64, buffer: *mut u8, buffer_len: usize,
) {
    let mut slice = std::slice::from_raw_parts_mut(buffer, buffer_len);
    let t = OffsetDateTime::from_unix_timestamp(nsecs).unwrap();
    t.format_into(&mut slice, &Rfc3339).unwrap();
}
