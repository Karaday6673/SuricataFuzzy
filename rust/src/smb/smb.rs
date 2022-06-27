/* Copyright (C) 2017-2020 Open Information Security Foundation
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

/* TODO
 * - check all parsers for calls on non-SUCCESS status
 */

/*  GAP processing:
 *  - if post-gap we've seen a succesful tx req/res: we consider "re-sync'd"
 */

// written by Victor Julien

use std;
use std::str;
use std::ffi::{self, CString};

use std::collections::HashMap;

use nom7::{Err, Needed};
use nom7::error::{make_error, ErrorKind};

use crate::core::*;
use crate::applayer;
use crate::applayer::*;
use crate::frames::*;
use crate::conf::*;
use crate::filecontainer::*;
use crate::applayer::{AppLayerResult, AppLayerTxData, AppLayerEvent};

use crate::smb::nbss_records::*;
use crate::smb::smb1_records::*;
use crate::smb::smb2_records::*;

use crate::smb::smb1::*;
use crate::smb::smb2::*;
use crate::smb::smb3::*;
use crate::smb::dcerpc::*;
use crate::smb::session::*;
use crate::smb::events::*;
use crate::smb::files::*;
use crate::smb::smb2_ioctl::*;

#[derive(AppLayerFrameType)]
pub enum SMBFrameType {
    NBSSPdu,
    NBSSHdr,
    NBSSData,
    SMB1Pdu,
    SMB1Hdr,
    SMB1Data,
    SMB2Pdu,
    SMB2Hdr,
    SMB2Data,
    SMB3Pdu,
    SMB3Hdr,
    SMB3Data,
}

pub const MIN_REC_SIZE: u16 = 32 + 4; // SMB hdr + nbss hdr
pub const SMB_CONFIG_DEFAULT_STREAM_DEPTH: u32 = 0;

pub static mut SMB_CFG_MAX_READ_SIZE: u32 = 0;
pub static mut SMB_CFG_MAX_READ_QUEUE_SIZE: u32 = 0;
pub static mut SMB_CFG_MAX_READ_QUEUE_CNT: u32 = 0;
pub static mut SMB_CFG_MAX_WRITE_SIZE: u32 = 0;
pub static mut SMB_CFG_MAX_WRITE_QUEUE_SIZE: u32 = 0;
pub static mut SMB_CFG_MAX_WRITE_QUEUE_CNT: u32 = 0;

static mut ALPROTO_SMB: AppProto = ALPROTO_UNKNOWN;

pub static mut SURICATA_SMB_FILE_CONFIG: Option<&'static SuricataFileContext> = None;

#[no_mangle]
pub extern "C" fn rs_smb_init(context: &'static mut SuricataFileContext)
{
    unsafe {
        SURICATA_SMB_FILE_CONFIG = Some(context);
    }
}

pub const SMB_NTSTATUS_SUCCESS:                                                        u32 = 0x00000000;
pub const SMB_NTSTATUS_WAIT_1:                                                         u32 = 0x00000001;
pub const SMB_NTSTATUS_WAIT_2:                                                         u32 = 0x00000002;
pub const SMB_NTSTATUS_WAIT_3:                                                         u32 = 0x00000003;
pub const SMB_NTSTATUS_WAIT_63:                                                        u32 = 0x0000003f;
pub const SMB_NTSTATUS_ABANDONED:                                                      u32 = 0x00000080;
pub const SMB_NTSTATUS_ABANDONED_WAIT_63:                                              u32 = 0x000000bf;
pub const SMB_NTSTATUS_USER_APC:                                                       u32 = 0x000000c0;
pub const SMB_NTSTATUS_ALERTED:                                                        u32 = 0x00000101;
pub const SMB_NTSTATUS_TIMEOUT:                                                        u32 = 0x00000102;
pub const SMB_NTSTATUS_PENDING:                                                        u32 = 0x00000103;
pub const SMB_NTSTATUS_REPARSE:                                                        u32 = 0x00000104;
pub const SMB_NTSTATUS_MORE_ENTRIES:                                                   u32 = 0x00000105;
pub const SMB_NTSTATUS_NOT_ALL_ASSIGNED:                                               u32 = 0x00000106;
pub const SMB_NTSTATUS_SOME_NOT_MAPPED:                                                u32 = 0x00000107;
pub const SMB_NTSTATUS_OPLOCK_BREAK_IN_PROGRESS:                                       u32 = 0x00000108;
pub const SMB_NTSTATUS_VOLUME_MOUNTED:                                                 u32 = 0x00000109;
pub const SMB_NTSTATUS_RXACT_COMMITTED:                                                u32 = 0x0000010a;
pub const SMB_NTSTATUS_NOTIFY_CLEANUP:                                                 u32 = 0x0000010b;
pub const SMB_NTSTATUS_NOTIFY_ENUM_DIR:                                                u32 = 0x0000010c;
pub const SMB_NTSTATUS_NO_QUOTAS_FOR_ACCOUNT:                                          u32 = 0x0000010d;
pub const SMB_NTSTATUS_PRIMARY_TRANSPORT_CONNECT_FAILED:                               u32 = 0x0000010e;
pub const SMB_NTSTATUS_PAGE_FAULT_TRANSITION:                                          u32 = 0x00000110;
pub const SMB_NTSTATUS_PAGE_FAULT_DEMAND_ZERO:                                         u32 = 0x00000111;
pub const SMB_NTSTATUS_PAGE_FAULT_COPY_ON_WRITE:                                       u32 = 0x00000112;
pub const SMB_NTSTATUS_PAGE_FAULT_GUARD_PAGE:                                          u32 = 0x00000113;
pub const SMB_NTSTATUS_PAGE_FAULT_PAGING_FILE:                                         u32 = 0x00000114;
pub const SMB_NTSTATUS_CACHE_PAGE_LOCKED:                                              u32 = 0x00000115;
pub const SMB_NTSTATUS_CRASH_DUMP:                                                     u32 = 0x00000116;
pub const SMB_NTSTATUS_BUFFER_ALL_ZEROS:                                               u32 = 0x00000117;
pub const SMB_NTSTATUS_REPARSE_OBJECT:                                                 u32 = 0x00000118;
pub const SMB_NTSTATUS_RESOURCE_REQUIREMENTS_CHANGED:                                  u32 = 0x00000119;
pub const SMB_NTSTATUS_TRANSLATION_COMPLETE:                                           u32 = 0x00000120;
pub const SMB_NTSTATUS_DS_MEMBERSHIP_EVALUATED_LOCALLY:                                u32 = 0x00000121;
pub const SMB_NTSTATUS_NOTHING_TO_TERMINATE:                                           u32 = 0x00000122;
pub const SMB_NTSTATUS_PROCESS_NOT_IN_JOB:                                             u32 = 0x00000123;
pub const SMB_NTSTATUS_PROCESS_IN_JOB:                                                 u32 = 0x00000124;
pub const SMB_NTSTATUS_VOLSNAP_HIBERNATE_READY:                                        u32 = 0x00000125;
pub const SMB_NTSTATUS_FSFILTER_OP_COMPLETED_SUCCESSFULLY:                             u32 = 0x00000126;
pub const SMB_NTSTATUS_INTERRUPT_VECTOR_ALREADY_CONNECTED:                             u32 = 0x00000127;
pub const SMB_NTSTATUS_INTERRUPT_STILL_CONNECTED:                                      u32 = 0x00000128;
pub const SMB_NTSTATUS_PROCESS_CLONED:                                                 u32 = 0x00000129;
pub const SMB_NTSTATUS_FILE_LOCKED_WITH_ONLY_READERS:                                  u32 = 0x0000012a;
pub const SMB_NTSTATUS_FILE_LOCKED_WITH_WRITERS:                                       u32 = 0x0000012b;
pub const SMB_NTSTATUS_RESOURCEMANAGER_READ_ONLY:                                      u32 = 0x00000202;
pub const SMB_NTSTATUS_WAIT_FOR_OPLOCK:                                                u32 = 0x00000367;
pub const SMB_NTDBG_EXCEPTION_HANDLED:                                                 u32 = 0x00010001;
pub const SMB_NTDBG_CONTINUE:                                                          u32 = 0x00010002;
pub const SMB_NTSTATUS_FLT_IO_COMPLETE:                                                u32 = 0x001c0001;
pub const SMB_NTSTATUS_FILE_NOT_AVAILABLE:                                             u32 = 0xc0000467;
pub const SMB_NTSTATUS_SHARE_UNAVAILABLE:                                              u32 = 0xc0000480;
pub const SMB_NTSTATUS_CALLBACK_RETURNED_THREAD_AFFINITY:                              u32 = 0xc0000721;
pub const SMB_NTSTATUS_OBJECT_NAME_EXISTS:                                             u32 = 0x40000000;
pub const SMB_NTSTATUS_THREAD_WAS_SUSPENDED:                                           u32 = 0x40000001;
pub const SMB_NTSTATUS_WORKING_SET_LIMIT_RANGE:                                        u32 = 0x40000002;
pub const SMB_NTSTATUS_IMAGE_NOT_AT_BASE:                                              u32 = 0x40000003;
pub const SMB_NTSTATUS_RXACT_STATE_CREATED:                                            u32 = 0x40000004;
pub const SMB_NTSTATUS_SEGMENT_NOTIFICATION:                                           u32 = 0x40000005;
pub const SMB_NTSTATUS_LOCAL_USER_SESSION_KEY:                                         u32 = 0x40000006;
pub const SMB_NTSTATUS_BAD_CURRENT_DIRECTORY:                                          u32 = 0x40000007;
pub const SMB_NTSTATUS_SERIAL_MORE_WRITES:                                             u32 = 0x40000008;
pub const SMB_NTSTATUS_REGISTRY_RECOVERED:                                             u32 = 0x40000009;
pub const SMB_NTSTATUS_FT_READ_RECOVERY_FROM_BACKUP:                                   u32 = 0x4000000a;
pub const SMB_NTSTATUS_FT_WRITE_RECOVERY:                                              u32 = 0x4000000b;
pub const SMB_NTSTATUS_SERIAL_COUNTER_TIMEOUT:                                         u32 = 0x4000000c;
pub const SMB_NTSTATUS_NULL_LM_PASSWORD:                                               u32 = 0x4000000d;
pub const SMB_NTSTATUS_IMAGE_MACHINE_TYPE_MISMATCH:                                    u32 = 0x4000000e;
pub const SMB_NTSTATUS_RECEIVE_PARTIAL:                                                u32 = 0x4000000f;
pub const SMB_NTSTATUS_RECEIVE_EXPEDITED:                                              u32 = 0x40000010;
pub const SMB_NTSTATUS_RECEIVE_PARTIAL_EXPEDITED:                                      u32 = 0x40000011;
pub const SMB_NTSTATUS_EVENT_DONE:                                                     u32 = 0x40000012;
pub const SMB_NTSTATUS_EVENT_PENDING:                                                  u32 = 0x40000013;
pub const SMB_NTSTATUS_CHECKING_FILE_SYSTEM:                                           u32 = 0x40000014;
pub const SMB_NTSTATUS_FATAL_APP_EXIT:                                                 u32 = 0x40000015;
pub const SMB_NTSTATUS_PREDEFINED_HANDLE:                                              u32 = 0x40000016;
pub const SMB_NTSTATUS_WAS_UNLOCKED:                                                   u32 = 0x40000017;
pub const SMB_NTSTATUS_SERVICE_NOTIFICATION:                                           u32 = 0x40000018;
pub const SMB_NTSTATUS_WAS_LOCKED:                                                     u32 = 0x40000019;
pub const SMB_NTSTATUS_LOG_HARD_ERROR:                                                 u32 = 0x4000001a;
pub const SMB_NTSTATUS_ALREADY_WIN32:                                                  u32 = 0x4000001b;
pub const SMB_NTSTATUS_WX86_UNSIMULATE:                                                u32 = 0x4000001c;
pub const SMB_NTSTATUS_WX86_CONTINUE:                                                  u32 = 0x4000001d;
pub const SMB_NTSTATUS_WX86_SINGLE_STEP:                                               u32 = 0x4000001e;
pub const SMB_NTSTATUS_WX86_BREAKPOINT:                                                u32 = 0x4000001f;
pub const SMB_NTSTATUS_WX86_EXCEPTION_CONTINUE:                                        u32 = 0x40000020;
pub const SMB_NTSTATUS_WX86_EXCEPTION_LASTCHANCE:                                      u32 = 0x40000021;
pub const SMB_NTSTATUS_WX86_EXCEPTION_CHAIN:                                           u32 = 0x40000022;
pub const SMB_NTSTATUS_IMAGE_MACHINE_TYPE_MISMATCH_EXE:                                u32 = 0x40000023;
pub const SMB_NTSTATUS_NO_YIELD_PERFORMED:                                             u32 = 0x40000024;
pub const SMB_NTSTATUS_TIMER_RESUME_IGNORED:                                           u32 = 0x40000025;
pub const SMB_NTSTATUS_ARBITRATION_UNHANDLED:                                          u32 = 0x40000026;
pub const SMB_NTSTATUS_CARDBUS_NOT_SUPPORTED:                                          u32 = 0x40000027;
pub const SMB_NTSTATUS_WX86_CREATEWX86TIB:                                             u32 = 0x40000028;
pub const SMB_NTSTATUS_MP_PROCESSOR_MISMATCH:                                          u32 = 0x40000029;
pub const SMB_NTSTATUS_HIBERNATED:                                                     u32 = 0x4000002a;
pub const SMB_NTSTATUS_RESUME_HIBERNATION:                                             u32 = 0x4000002b;
pub const SMB_NTSTATUS_FIRMWARE_UPDATED:                                               u32 = 0x4000002c;
pub const SMB_NTSTATUS_DRIVERS_LEAKING_LOCKED_PAGES:                                   u32 = 0x4000002d;
pub const SMB_NTSTATUS_MESSAGE_RETRIEVED:                                              u32 = 0x4000002e;
pub const SMB_NTSTATUS_SYSTEM_POWERSTATE_TRANSITION:                                   u32 = 0x4000002f;
pub const SMB_NTSTATUS_ALPC_CHECK_COMPLETION_LIST:                                     u32 = 0x40000030;
pub const SMB_NTSTATUS_SYSTEM_POWERSTATE_COMPLEX_TRANSITION:                           u32 = 0x40000031;
pub const SMB_NTSTATUS_ACCESS_AUDIT_BY_POLICY:                                         u32 = 0x40000032;
pub const SMB_NTSTATUS_ABANDON_HIBERFILE:                                              u32 = 0x40000033;
pub const SMB_NTSTATUS_BIZRULES_NOT_ENABLED:                                           u32 = 0x40000034;
pub const SMB_NTSTATUS_WAKE_SYSTEM:                                                    u32 = 0x40000294;
pub const SMB_NTSTATUS_DS_SHUTTING_DOWN:                                               u32 = 0x40000370;
pub const SMB_NTDBG_REPLY_LATER:                                                       u32 = 0x40010001;
pub const SMB_NTDBG_UNABLE_TO_PROVIDE_HANDLE:                                          u32 = 0x40010002;
pub const SMB_NTDBG_TERMINATE_THREAD:                                                  u32 = 0x40010003;
pub const SMB_NTDBG_TERMINATE_PROCESS:                                                 u32 = 0x40010004;
pub const SMB_NTDBG_CONTROL_C:                                                         u32 = 0x40010005;
pub const SMB_NTDBG_PRINTEXCEPTION_C:                                                  u32 = 0x40010006;
pub const SMB_NTDBG_RIPEXCEPTION:                                                      u32 = 0x40010007;
pub const SMB_NTDBG_CONTROL_BREAK:                                                     u32 = 0x40010008;
pub const SMB_NTDBG_COMMAND_EXCEPTION:                                                 u32 = 0x40010009;
pub const SMB_NTRPC_NT_UUID_LOCAL_ONLY:                                                u32 = 0x40020056;
pub const SMB_NTRPC_NT_SEND_INCOMPLETE:                                                u32 = 0x400200af;
pub const SMB_NTSTATUS_CTX_CDM_CONNECT:                                                u32 = 0x400a0004;
pub const SMB_NTSTATUS_CTX_CDM_DISCONNECT:                                             u32 = 0x400a0005;
pub const SMB_NTSTATUS_SXS_RELEASE_ACTIVATION_CONTEXT:                                 u32 = 0x4015000d;
pub const SMB_NTSTATUS_RECOVERY_NOT_NEEDED:                                            u32 = 0x40190034;
pub const SMB_NTSTATUS_RM_ALREADY_STARTED:                                             u32 = 0x40190035;
pub const SMB_NTSTATUS_LOG_NO_RESTART:                                                 u32 = 0x401a000c;
pub const SMB_NTSTATUS_VIDEO_DRIVER_DEBUG_REPORT_REQUEST:                              u32 = 0x401b00ec;
pub const SMB_NTSTATUS_GRAPHICS_PARTIAL_DATA_POPULATED:                                u32 = 0x401e000a;
pub const SMB_NTSTATUS_GRAPHICS_DRIVER_MISMATCH:                                       u32 = 0x401e0117;
pub const SMB_NTSTATUS_GRAPHICS_MODE_NOT_PINNED:                                       u32 = 0x401e0307;
pub const SMB_NTSTATUS_GRAPHICS_NO_PREFERRED_MODE:                                     u32 = 0x401e031e;
pub const SMB_NTSTATUS_GRAPHICS_DATASET_IS_EMPTY:                                      u32 = 0x401e034b;
pub const SMB_NTSTATUS_GRAPHICS_NO_MORE_ELEMENTS_IN_DATASET:                           u32 = 0x401e034c;
pub const SMB_NTSTATUS_GRAPHICS_PATH_CONTENT_GEOMETRY_TRANSFORMATION_NOT_PINNED:       u32 = 0x401e0351;
pub const SMB_NTSTATUS_GRAPHICS_UNKNOWN_CHILD_STATUS:                                  u32 = 0x401e042f;
pub const SMB_NTSTATUS_GRAPHICS_LEADLINK_START_DEFERRED:                               u32 = 0x401e0437;
pub const SMB_NTSTATUS_GRAPHICS_POLLING_TOO_FREQUENTLY:                                u32 = 0x401e0439;
pub const SMB_NTSTATUS_GRAPHICS_START_DEFERRED:                                        u32 = 0x401e043a;
pub const SMB_NTSTATUS_NDIS_INDICATION_REQUIRED:                                       u32 = 0x40230001;
pub const SMB_NTSTATUS_GUARD_PAGE_VIOLATION:                                           u32 = 0x80000001;
pub const SMB_NTSTATUS_DATATYPE_MISALIGNMENT:                                          u32 = 0x80000002;
pub const SMB_NTSTATUS_BREAKPOINT:                                                     u32 = 0x80000003;
pub const SMB_NTSTATUS_SINGLE_STEP:                                                    u32 = 0x80000004;
pub const SMB_NTSTATUS_BUFFER_OVERFLOW:                                                u32 = 0x80000005;
pub const SMB_NTSTATUS_NO_MORE_FILES:                                                  u32 = 0x80000006;
pub const SMB_NTSTATUS_WAKE_SYSTEM_DEBUGGER:                                           u32 = 0x80000007;
pub const SMB_NTSTATUS_HANDLES_CLOSED:                                                 u32 = 0x8000000a;
pub const SMB_NTSTATUS_NO_INHERITANCE:                                                 u32 = 0x8000000b;
pub const SMB_NTSTATUS_GUID_SUBSTITUTION_MADE:                                         u32 = 0x8000000c;
pub const SMB_NTSTATUS_PARTIAL_COPY:                                                   u32 = 0x8000000d;
pub const SMB_NTSTATUS_DEVICE_PAPER_EMPTY:                                             u32 = 0x8000000e;
pub const SMB_NTSTATUS_DEVICE_POWERED_OFF:                                             u32 = 0x8000000f;
pub const SMB_NTSTATUS_DEVICE_OFF_LINE:                                                u32 = 0x80000010;
pub const SMB_NTSTATUS_DEVICE_BUSY:                                                    u32 = 0x80000011;
pub const SMB_NTSTATUS_NO_MORE_EAS:                                                    u32 = 0x80000012;
pub const SMB_NTSTATUS_INVALID_EA_NAME:                                                u32 = 0x80000013;
pub const SMB_NTSTATUS_EA_LIST_INCONSISTENT:                                           u32 = 0x80000014;
pub const SMB_NTSTATUS_INVALID_EA_FLAG:                                                u32 = 0x80000015;
pub const SMB_NTSTATUS_VERIFY_REQUIRED:                                                u32 = 0x80000016;
pub const SMB_NTSTATUS_EXTRANEOUS_INFORMATION:                                         u32 = 0x80000017;
pub const SMB_NTSTATUS_RXACT_COMMIT_NECESSARY:                                         u32 = 0x80000018;
pub const SMB_NTSTATUS_NO_MORE_ENTRIES:                                                u32 = 0x8000001a;
pub const SMB_NTSTATUS_FILEMARK_DETECTED:                                              u32 = 0x8000001b;
pub const SMB_NTSTATUS_MEDIA_CHANGED:                                                  u32 = 0x8000001c;
pub const SMB_NTSTATUS_BUS_RESET:                                                      u32 = 0x8000001d;
pub const SMB_NTSTATUS_END_OF_MEDIA:                                                   u32 = 0x8000001e;
pub const SMB_NTSTATUS_BEGINNING_OF_MEDIA:                                             u32 = 0x8000001f;
pub const SMB_NTSTATUS_MEDIA_CHECK:                                                    u32 = 0x80000020;
pub const SMB_NTSTATUS_SETMARK_DETECTED:                                               u32 = 0x80000021;
pub const SMB_NTSTATUS_NO_DATA_DETECTED:                                               u32 = 0x80000022;
pub const SMB_NTSTATUS_REDIRECTOR_HAS_OPEN_HANDLES:                                    u32 = 0x80000023;
pub const SMB_NTSTATUS_SERVER_HAS_OPEN_HANDLES:                                        u32 = 0x80000024;
pub const SMB_NTSTATUS_ALREADY_DISCONNECTED:                                           u32 = 0x80000025;
pub const SMB_NTSTATUS_LONGJUMP:                                                       u32 = 0x80000026;
pub const SMB_NTSTATUS_CLEANER_CARTRIDGE_INSTALLED:                                    u32 = 0x80000027;
pub const SMB_NTSTATUS_PLUGPLAY_QUERY_VETOED:                                          u32 = 0x80000028;
pub const SMB_NTSTATUS_UNWIND_CONSOLIDATE:                                             u32 = 0x80000029;
pub const SMB_NTSTATUS_REGISTRY_HIVE_RECOVERED:                                        u32 = 0x8000002a;
pub const SMB_NTSTATUS_DLL_MIGHT_BE_INSECURE:                                          u32 = 0x8000002b;
pub const SMB_NTSTATUS_DLL_MIGHT_BE_INCOMPATIBLE:                                      u32 = 0x8000002c;
pub const SMB_NTSTATUS_STOPPED_ON_SYMLINK:                                             u32 = 0x8000002d;
pub const SMB_NTSTATUS_DEVICE_REQUIRES_CLEANING:                                       u32 = 0x80000288;
pub const SMB_NTSTATUS_DEVICE_DOOR_OPEN:                                               u32 = 0x80000289;
pub const SMB_NTSTATUS_DATA_LOST_REPAIR:                                               u32 = 0x80000803;
pub const SMB_NTDBG_EXCEPTION_NOT_HANDLED:                                             u32 = 0x80010001;
pub const SMB_NTSTATUS_CLUSTER_NODE_ALREADY_UP:                                        u32 = 0x80130001;
pub const SMB_NTSTATUS_CLUSTER_NODE_ALREADY_DOWN:                                      u32 = 0x80130002;
pub const SMB_NTSTATUS_CLUSTER_NETWORK_ALREADY_ONLINE:                                 u32 = 0x80130003;
pub const SMB_NTSTATUS_CLUSTER_NETWORK_ALREADY_OFFLINE:                                u32 = 0x80130004;
pub const SMB_NTSTATUS_CLUSTER_NODE_ALREADY_MEMBER:                                    u32 = 0x80130005;
pub const SMB_NTSTATUS_COULD_NOT_RESIZE_LOG:                                           u32 = 0x80190009;
pub const SMB_NTSTATUS_NO_TXF_METADATA:                                                u32 = 0x80190029;
pub const SMB_NTSTATUS_CANT_RECOVER_WITH_HANDLE_OPEN:                                  u32 = 0x80190031;
pub const SMB_NTSTATUS_TXF_METADATA_ALREADY_PRESENT:                                   u32 = 0x80190041;
pub const SMB_NTSTATUS_TRANSACTION_SCOPE_CALLBACKS_NOT_SET:                            u32 = 0x80190042;
pub const SMB_NTSTATUS_VIDEO_HUNG_DISPLAY_DRIVER_THREAD_RECOVERED:                     u32 = 0x801b00eb;
pub const SMB_NTSTATUS_FLT_BUFFER_TOO_SMALL:                                           u32 = 0x801c0001;
pub const SMB_NTSTATUS_FVE_PARTIAL_METADATA:                                           u32 = 0x80210001;
pub const SMB_NTSTATUS_FVE_TRANSIENT_STATE:                                            u32 = 0x80210002;
pub const SMB_NTSTATUS_UNSUCCESSFUL:                                                   u32 = 0xc0000001;
pub const SMB_NTSTATUS_NOT_IMPLEMENTED:                                                u32 = 0xc0000002;
pub const SMB_NTSTATUS_INVALID_INFO_CLASS:                                             u32 = 0xc0000003;
pub const SMB_NTSTATUS_INFO_LENGTH_MISMATCH:                                           u32 = 0xc0000004;
pub const SMB_NTSTATUS_ACCESS_VIOLATION:                                               u32 = 0xc0000005;
pub const SMB_NTSTATUS_IN_PAGE_ERROR:                                                  u32 = 0xc0000006;
pub const SMB_NTSTATUS_PAGEFILE_QUOTA:                                                 u32 = 0xc0000007;
pub const SMB_NTSTATUS_INVALID_HANDLE:                                                 u32 = 0xc0000008;
pub const SMB_NTSTATUS_BAD_INITIAL_STACK:                                              u32 = 0xc0000009;
pub const SMB_NTSTATUS_BAD_INITIAL_PC:                                                 u32 = 0xc000000a;
pub const SMB_NTSTATUS_INVALID_CID:                                                    u32 = 0xc000000b;
pub const SMB_NTSTATUS_TIMER_NOT_CANCELED:                                             u32 = 0xc000000c;
pub const SMB_NTSTATUS_INVALID_PARAMETER:                                              u32 = 0xc000000d;
pub const SMB_NTSTATUS_NO_SUCH_DEVICE:                                                 u32 = 0xc000000e;
pub const SMB_NTSTATUS_NO_SUCH_FILE:                                                   u32 = 0xc000000f;
pub const SMB_NTSTATUS_INVALID_DEVICE_REQUEST:                                         u32 = 0xc0000010;
pub const SMB_NTSTATUS_END_OF_FILE:                                                    u32 = 0xc0000011;
pub const SMB_NTSTATUS_WRONG_VOLUME:                                                   u32 = 0xc0000012;
pub const SMB_NTSTATUS_NO_MEDIA_IN_DEVICE:                                             u32 = 0xc0000013;
pub const SMB_NTSTATUS_UNRECOGNIZED_MEDIA:                                             u32 = 0xc0000014;
pub const SMB_NTSTATUS_NONEXISTENT_SECTOR:                                             u32 = 0xc0000015;
pub const SMB_NTSTATUS_MORE_PROCESSING_REQUIRED:                                       u32 = 0xc0000016;
pub const SMB_NTSTATUS_NO_MEMORY:                                                      u32 = 0xc0000017;
pub const SMB_NTSTATUS_CONFLICTING_ADDRESSES:                                          u32 = 0xc0000018;
pub const SMB_NTSTATUS_NOT_MAPPED_VIEW:                                                u32 = 0xc0000019;
pub const SMB_NTSTATUS_UNABLE_TO_FREE_VM:                                              u32 = 0xc000001a;
pub const SMB_NTSTATUS_UNABLE_TO_DELETE_SECTION:                                       u32 = 0xc000001b;
pub const SMB_NTSTATUS_INVALID_SYSTEM_SERVICE:                                         u32 = 0xc000001c;
pub const SMB_NTSTATUS_ILLEGAL_INSTRUCTION:                                            u32 = 0xc000001d;
pub const SMB_NTSTATUS_INVALID_LOCK_SEQUENCE:                                          u32 = 0xc000001e;
pub const SMB_NTSTATUS_INVALID_VIEW_SIZE:                                              u32 = 0xc000001f;
pub const SMB_NTSTATUS_INVALID_FILE_FOR_SECTION:                                       u32 = 0xc0000020;
pub const SMB_NTSTATUS_ALREADY_COMMITTED:                                              u32 = 0xc0000021;
pub const SMB_NTSTATUS_ACCESS_DENIED:                                                  u32 = 0xc0000022;
pub const SMB_NTSTATUS_BUFFER_TOO_SMALL:                                               u32 = 0xc0000023;
pub const SMB_NTSTATUS_OBJECT_TYPE_MISMATCH:                                           u32 = 0xc0000024;
pub const SMB_NTSTATUS_NONCONTINUABLE_EXCEPTION:                                       u32 = 0xc0000025;
pub const SMB_NTSTATUS_INVALID_DISPOSITION:                                            u32 = 0xc0000026;
pub const SMB_NTSTATUS_UNWIND:                                                         u32 = 0xc0000027;
pub const SMB_NTSTATUS_BAD_STACK:                                                      u32 = 0xc0000028;
pub const SMB_NTSTATUS_INVALID_UNWIND_TARGET:                                          u32 = 0xc0000029;
pub const SMB_NTSTATUS_NOT_LOCKED:                                                     u32 = 0xc000002a;
pub const SMB_NTSTATUS_PARITY_ERROR:                                                   u32 = 0xc000002b;
pub const SMB_NTSTATUS_UNABLE_TO_DECOMMIT_VM:                                          u32 = 0xc000002c;
pub const SMB_NTSTATUS_NOT_COMMITTED:                                                  u32 = 0xc000002d;
pub const SMB_NTSTATUS_INVALID_PORT_ATTRIBUTES:                                        u32 = 0xc000002e;
pub const SMB_NTSTATUS_PORT_MESSAGE_TOO_LONG:                                          u32 = 0xc000002f;
pub const SMB_NTSTATUS_INVALID_PARAMETER_MIX:                                          u32 = 0xc0000030;
pub const SMB_NTSTATUS_INVALID_QUOTA_LOWER:                                            u32 = 0xc0000031;
pub const SMB_NTSTATUS_DISK_CORRUPT_ERROR:                                             u32 = 0xc0000032;
pub const SMB_NTSTATUS_OBJECT_NAME_INVALID:                                            u32 = 0xc0000033;
pub const SMB_NTSTATUS_OBJECT_NAME_NOT_FOUND:                                          u32 = 0xc0000034;
pub const SMB_NTSTATUS_OBJECT_NAME_COLLISION:                                          u32 = 0xc0000035;
pub const SMB_NTSTATUS_PORT_DISCONNECTED:                                              u32 = 0xc0000037;
pub const SMB_NTSTATUS_DEVICE_ALREADY_ATTACHED:                                        u32 = 0xc0000038;
pub const SMB_NTSTATUS_OBJECT_PATH_INVALID:                                            u32 = 0xc0000039;
pub const SMB_NTSTATUS_OBJECT_PATH_NOT_FOUND:                                          u32 = 0xc000003a;
pub const SMB_NTSTATUS_OBJECT_PATH_SYNTAX_BAD:                                         u32 = 0xc000003b;
pub const SMB_NTSTATUS_DATA_OVERRUN:                                                   u32 = 0xc000003c;
pub const SMB_NTSTATUS_DATA_LATE_ERROR:                                                u32 = 0xc000003d;
pub const SMB_NTSTATUS_DATA_ERROR:                                                     u32 = 0xc000003e;
pub const SMB_NTSTATUS_CRC_ERROR:                                                      u32 = 0xc000003f;
pub const SMB_NTSTATUS_SECTION_TOO_BIG:                                                u32 = 0xc0000040;
pub const SMB_NTSTATUS_PORT_CONNECTION_REFUSED:                                        u32 = 0xc0000041;
pub const SMB_NTSTATUS_INVALID_PORT_HANDLE:                                            u32 = 0xc0000042;
pub const SMB_NTSTATUS_SHARING_VIOLATION:                                              u32 = 0xc0000043;
pub const SMB_NTSTATUS_QUOTA_EXCEEDED:                                                 u32 = 0xc0000044;
pub const SMB_NTSTATUS_INVALID_PAGE_PROTECTION:                                        u32 = 0xc0000045;
pub const SMB_NTSTATUS_MUTANT_NOT_OWNED:                                               u32 = 0xc0000046;
pub const SMB_NTSTATUS_SEMAPHORE_LIMIT_EXCEEDED:                                       u32 = 0xc0000047;
pub const SMB_NTSTATUS_PORT_ALREADY_SET:                                               u32 = 0xc0000048;
pub const SMB_NTSTATUS_SECTION_NOT_IMAGE:                                              u32 = 0xc0000049;
pub const SMB_NTSTATUS_SUSPEND_COUNT_EXCEEDED:                                         u32 = 0xc000004a;
pub const SMB_NTSTATUS_THREAD_IS_TERMINATING:                                          u32 = 0xc000004b;
pub const SMB_NTSTATUS_BAD_WORKING_SET_LIMIT:                                          u32 = 0xc000004c;
pub const SMB_NTSTATUS_INCOMPATIBLE_FILE_MAP:                                          u32 = 0xc000004d;
pub const SMB_NTSTATUS_SECTION_PROTECTION:                                             u32 = 0xc000004e;
pub const SMB_NTSTATUS_EAS_NOT_SUPPORTED:                                              u32 = 0xc000004f;
pub const SMB_NTSTATUS_EA_TOO_LARGE:                                                   u32 = 0xc0000050;
pub const SMB_NTSTATUS_NONEXISTENT_EA_ENTRY:                                           u32 = 0xc0000051;
pub const SMB_NTSTATUS_NO_EAS_ON_FILE:                                                 u32 = 0xc0000052;
pub const SMB_NTSTATUS_EA_CORRUPT_ERROR:                                               u32 = 0xc0000053;
pub const SMB_NTSTATUS_FILE_LOCK_CONFLICT:                                             u32 = 0xc0000054;
pub const SMB_NTSTATUS_LOCK_NOT_GRANTED:                                               u32 = 0xc0000055;
pub const SMB_NTSTATUS_DELETE_PENDING:                                                 u32 = 0xc0000056;
pub const SMB_NTSTATUS_CTL_FILE_NOT_SUPPORTED:                                         u32 = 0xc0000057;
pub const SMB_NTSTATUS_UNKNOWN_REVISION:                                               u32 = 0xc0000058;
pub const SMB_NTSTATUS_REVISION_MISMATCH:                                              u32 = 0xc0000059;
pub const SMB_NTSTATUS_INVALID_OWNER:                                                  u32 = 0xc000005a;
pub const SMB_NTSTATUS_INVALID_PRIMARY_GROUP:                                          u32 = 0xc000005b;
pub const SMB_NTSTATUS_NO_IMPERSONATION_TOKEN:                                         u32 = 0xc000005c;
pub const SMB_NTSTATUS_CANT_DISABLE_MANDATORY:                                         u32 = 0xc000005d;
pub const SMB_NTSTATUS_NO_LOGON_SERVERS:                                               u32 = 0xc000005e;
pub const SMB_NTSTATUS_NO_SUCH_LOGON_SESSION:                                          u32 = 0xc000005f;
pub const SMB_NTSTATUS_NO_SUCH_PRIVILEGE:                                              u32 = 0xc0000060;
pub const SMB_NTSTATUS_PRIVILEGE_NOT_HELD:                                             u32 = 0xc0000061;
pub const SMB_NTSTATUS_INVALID_ACCOUNT_NAME:                                           u32 = 0xc0000062;
pub const SMB_NTSTATUS_USER_EXISTS:                                                    u32 = 0xc0000063;
pub const SMB_NTSTATUS_NO_SUCH_USER:                                                   u32 = 0xc0000064;
pub const SMB_NTSTATUS_GROUP_EXISTS:                                                   u32 = 0xc0000065;
pub const SMB_NTSTATUS_NO_SUCH_GROUP:                                                  u32 = 0xc0000066;
pub const SMB_NTSTATUS_MEMBER_IN_GROUP:                                                u32 = 0xc0000067;
pub const SMB_NTSTATUS_MEMBER_NOT_IN_GROUP:                                            u32 = 0xc0000068;
pub const SMB_NTSTATUS_LAST_ADMIN:                                                     u32 = 0xc0000069;
pub const SMB_NTSTATUS_WRONG_PASSWORD:                                                 u32 = 0xc000006a;
pub const SMB_NTSTATUS_ILL_FORMED_PASSWORD:                                            u32 = 0xc000006b;
pub const SMB_NTSTATUS_PASSWORD_RESTRICTION:                                           u32 = 0xc000006c;
pub const SMB_NTSTATUS_LOGON_FAILURE:                                                  u32 = 0xc000006d;
pub const SMB_NTSTATUS_ACCOUNT_RESTRICTION:                                            u32 = 0xc000006e;
pub const SMB_NTSTATUS_INVALID_LOGON_HOURS:                                            u32 = 0xc000006f;
pub const SMB_NTSTATUS_INVALID_WORKSTATION:                                            u32 = 0xc0000070;
pub const SMB_NTSTATUS_PASSWORD_EXPIRED:                                               u32 = 0xc0000071;
pub const SMB_NTSTATUS_ACCOUNT_DISABLED:                                               u32 = 0xc0000072;
pub const SMB_NTSTATUS_NONE_MAPPED:                                                    u32 = 0xc0000073;
pub const SMB_NTSTATUS_TOO_MANY_LUIDS_REQUESTED:                                       u32 = 0xc0000074;
pub const SMB_NTSTATUS_LUIDS_EXHAUSTED:                                                u32 = 0xc0000075;
pub const SMB_NTSTATUS_INVALID_SUB_AUTHORITY:                                          u32 = 0xc0000076;
pub const SMB_NTSTATUS_INVALID_ACL:                                                    u32 = 0xc0000077;
pub const SMB_NTSTATUS_INVALID_SID:                                                    u32 = 0xc0000078;
pub const SMB_NTSTATUS_INVALID_SECURITY_DESCR:                                         u32 = 0xc0000079;
pub const SMB_NTSTATUS_PROCEDURE_NOT_FOUND:                                            u32 = 0xc000007a;
pub const SMB_NTSTATUS_INVALID_IMAGE_FORMAT:                                           u32 = 0xc000007b;
pub const SMB_NTSTATUS_NO_TOKEN:                                                       u32 = 0xc000007c;
pub const SMB_NTSTATUS_BAD_INHERITANCE_ACL:                                            u32 = 0xc000007d;
pub const SMB_NTSTATUS_RANGE_NOT_LOCKED:                                               u32 = 0xc000007e;
pub const SMB_NTSTATUS_DISK_FULL:                                                      u32 = 0xc000007f;
pub const SMB_NTSTATUS_SERVER_DISABLED:                                                u32 = 0xc0000080;
pub const SMB_NTSTATUS_SERVER_NOT_DISABLED:                                            u32 = 0xc0000081;
pub const SMB_NTSTATUS_TOO_MANY_GUIDS_REQUESTED:                                       u32 = 0xc0000082;
pub const SMB_NTSTATUS_GUIDS_EXHAUSTED:                                                u32 = 0xc0000083;
pub const SMB_NTSTATUS_INVALID_ID_AUTHORITY:                                           u32 = 0xc0000084;
pub const SMB_NTSTATUS_AGENTS_EXHAUSTED:                                               u32 = 0xc0000085;
pub const SMB_NTSTATUS_INVALID_VOLUME_LABEL:                                           u32 = 0xc0000086;
pub const SMB_NTSTATUS_SECTION_NOT_EXTENDED:                                           u32 = 0xc0000087;
pub const SMB_NTSTATUS_NOT_MAPPED_DATA:                                                u32 = 0xc0000088;
pub const SMB_NTSTATUS_RESOURCE_DATA_NOT_FOUND:                                        u32 = 0xc0000089;
pub const SMB_NTSTATUS_RESOURCE_TYPE_NOT_FOUND:                                        u32 = 0xc000008a;
pub const SMB_NTSTATUS_RESOURCE_NAME_NOT_FOUND:                                        u32 = 0xc000008b;
pub const SMB_NTSTATUS_ARRAY_BOUNDS_EXCEEDED:                                          u32 = 0xc000008c;
pub const SMB_NTSTATUS_FLOAT_DENORMAL_OPERAND:                                         u32 = 0xc000008d;
pub const SMB_NTSTATUS_FLOAT_DIVIDE_BY_ZERO:                                           u32 = 0xc000008e;
pub const SMB_NTSTATUS_FLOAT_INEXACT_RESULT:                                           u32 = 0xc000008f;
pub const SMB_NTSTATUS_FLOAT_INVALID_OPERATION:                                        u32 = 0xc0000090;
pub const SMB_NTSTATUS_FLOAT_OVERFLOW:                                                 u32 = 0xc0000091;
pub const SMB_NTSTATUS_FLOAT_STACK_CHECK:                                              u32 = 0xc0000092;
pub const SMB_NTSTATUS_FLOAT_UNDERFLOW:                                                u32 = 0xc0000093;
pub const SMB_NTSTATUS_INTEGER_DIVIDE_BY_ZERO:                                         u32 = 0xc0000094;
pub const SMB_NTSTATUS_INTEGER_OVERFLOW:                                               u32 = 0xc0000095;
pub const SMB_NTSTATUS_PRIVILEGED_INSTRUCTION:                                         u32 = 0xc0000096;
pub const SMB_NTSTATUS_TOO_MANY_PAGING_FILES:                                          u32 = 0xc0000097;
pub const SMB_NTSTATUS_FILE_INVALID:                                                   u32 = 0xc0000098;
pub const SMB_NTSTATUS_ALLOTTED_SPACE_EXCEEDED:                                        u32 = 0xc0000099;
pub const SMB_NTSTATUS_INSUFFICIENT_RESOURCES:                                         u32 = 0xc000009a;
pub const SMB_NTSTATUS_DFS_EXIT_PATH_FOUND:                                            u32 = 0xc000009b;
pub const SMB_NTSTATUS_DEVICE_DATA_ERROR:                                              u32 = 0xc000009c;
pub const SMB_NTSTATUS_DEVICE_NOT_CONNECTED:                                           u32 = 0xc000009d;
pub const SMB_NTSTATUS_FREE_VM_NOT_AT_BASE:                                            u32 = 0xc000009f;
pub const SMB_NTSTATUS_MEMORY_NOT_ALLOCATED:                                           u32 = 0xc00000a0;
pub const SMB_NTSTATUS_WORKING_SET_QUOTA:                                              u32 = 0xc00000a1;
pub const SMB_NTSTATUS_MEDIA_WRITE_PROTECTED:                                          u32 = 0xc00000a2;
pub const SMB_NTSTATUS_DEVICE_NOT_READY:                                               u32 = 0xc00000a3;
pub const SMB_NTSTATUS_INVALID_GROUP_ATTRIBUTES:                                       u32 = 0xc00000a4;
pub const SMB_NTSTATUS_BAD_IMPERSONATION_LEVEL:                                        u32 = 0xc00000a5;
pub const SMB_NTSTATUS_CANT_OPEN_ANONYMOUS:                                            u32 = 0xc00000a6;
pub const SMB_NTSTATUS_BAD_VALIDATION_CLASS:                                           u32 = 0xc00000a7;
pub const SMB_NTSTATUS_BAD_TOKEN_TYPE:                                                 u32 = 0xc00000a8;
pub const SMB_NTSTATUS_BAD_MASTER_BOOT_RECORD:                                         u32 = 0xc00000a9;
pub const SMB_NTSTATUS_INSTRUCTION_MISALIGNMENT:                                       u32 = 0xc00000aa;
pub const SMB_NTSTATUS_INSTANCE_NOT_AVAILABLE:                                         u32 = 0xc00000ab;
pub const SMB_NTSTATUS_PIPE_NOT_AVAILABLE:                                             u32 = 0xc00000ac;
pub const SMB_NTSTATUS_INVALID_PIPE_STATE:                                             u32 = 0xc00000ad;
pub const SMB_NTSTATUS_PIPE_BUSY:                                                      u32 = 0xc00000ae;
pub const SMB_NTSTATUS_ILLEGAL_FUNCTION:                                               u32 = 0xc00000af;
pub const SMB_NTSTATUS_PIPE_DISCONNECTED:                                              u32 = 0xc00000b0;
pub const SMB_NTSTATUS_PIPE_CLOSING:                                                   u32 = 0xc00000b1;
pub const SMB_NTSTATUS_PIPE_CONNECTED:                                                 u32 = 0xc00000b2;
pub const SMB_NTSTATUS_PIPE_LISTENING:                                                 u32 = 0xc00000b3;
pub const SMB_NTSTATUS_INVALID_READ_MODE:                                              u32 = 0xc00000b4;
pub const SMB_NTSTATUS_IO_TIMEOUT:                                                     u32 = 0xc00000b5;
pub const SMB_NTSTATUS_FILE_FORCED_CLOSED:                                             u32 = 0xc00000b6;
pub const SMB_NTSTATUS_PROFILING_NOT_STARTED:                                          u32 = 0xc00000b7;
pub const SMB_NTSTATUS_PROFILING_NOT_STOPPED:                                          u32 = 0xc00000b8;
pub const SMB_NTSTATUS_COULD_NOT_INTERPRET:                                            u32 = 0xc00000b9;
pub const SMB_NTSTATUS_FILE_IS_A_DIRECTORY:                                            u32 = 0xc00000ba;
pub const SMB_NTSTATUS_NOT_SUPPORTED:                                                  u32 = 0xc00000bb;
pub const SMB_NTSTATUS_REMOTE_NOT_LISTENING:                                           u32 = 0xc00000bc;
pub const SMB_NTSTATUS_DUPLICATE_NAME:                                                 u32 = 0xc00000bd;
pub const SMB_NTSTATUS_BAD_NETWORK_PATH:                                               u32 = 0xc00000be;
pub const SMB_NTSTATUS_NETWORK_BUSY:                                                   u32 = 0xc00000bf;
pub const SMB_NTSTATUS_DEVICE_DOES_NOT_EXIST:                                          u32 = 0xc00000c0;
pub const SMB_NTSTATUS_TOO_MANY_COMMANDS:                                              u32 = 0xc00000c1;
pub const SMB_NTSTATUS_ADAPTER_HARDWARE_ERROR:                                         u32 = 0xc00000c2;
pub const SMB_NTSTATUS_INVALID_NETWORK_RESPONSE:                                       u32 = 0xc00000c3;
pub const SMB_NTSTATUS_UNEXPECTED_NETWORK_ERROR:                                       u32 = 0xc00000c4;
pub const SMB_NTSTATUS_BAD_REMOTE_ADAPTER:                                             u32 = 0xc00000c5;
pub const SMB_NTSTATUS_PRINT_QUEUE_FULL:                                               u32 = 0xc00000c6;
pub const SMB_NTSTATUS_NO_SPOOL_SPACE:                                                 u32 = 0xc00000c7;
pub const SMB_NTSTATUS_PRINT_CANCELLED:                                                u32 = 0xc00000c8;
pub const SMB_NTSTATUS_NETWORK_NAME_DELETED:                                           u32 = 0xc00000c9;
pub const SMB_NTSTATUS_NETWORK_ACCESS_DENIED:                                          u32 = 0xc00000ca;
pub const SMB_NTSTATUS_BAD_DEVICE_TYPE:                                                u32 = 0xc00000cb;
pub const SMB_NTSTATUS_BAD_NETWORK_NAME:                                               u32 = 0xc00000cc;
pub const SMB_NTSTATUS_TOO_MANY_NAMES:                                                 u32 = 0xc00000cd;
pub const SMB_NTSTATUS_TOO_MANY_SESSIONS:                                              u32 = 0xc00000ce;
pub const SMB_NTSTATUS_SHARING_PAUSED:                                                 u32 = 0xc00000cf;
pub const SMB_NTSTATUS_REQUEST_NOT_ACCEPTED:                                           u32 = 0xc00000d0;
pub const SMB_NTSTATUS_REDIRECTOR_PAUSED:                                              u32 = 0xc00000d1;
pub const SMB_NTSTATUS_NET_WRITE_FAULT:                                                u32 = 0xc00000d2;
pub const SMB_NTSTATUS_PROFILING_AT_LIMIT:                                             u32 = 0xc00000d3;
pub const SMB_NTSTATUS_NOT_SAME_DEVICE:                                                u32 = 0xc00000d4;
pub const SMB_NTSTATUS_FILE_RENAMED:                                                   u32 = 0xc00000d5;
pub const SMB_NTSTATUS_VIRTUAL_CIRCUIT_CLOSED:                                         u32 = 0xc00000d6;
pub const SMB_NTSTATUS_NO_SECURITY_ON_OBJECT:                                          u32 = 0xc00000d7;
pub const SMB_NTSTATUS_CANT_WAIT:                                                      u32 = 0xc00000d8;
pub const SMB_NTSTATUS_PIPE_EMPTY:                                                     u32 = 0xc00000d9;
pub const SMB_NTSTATUS_CANT_ACCESS_DOMAIN_INFO:                                        u32 = 0xc00000da;
pub const SMB_NTSTATUS_CANT_TERMINATE_SELF:                                            u32 = 0xc00000db;
pub const SMB_NTSTATUS_INVALID_SERVER_STATE:                                           u32 = 0xc00000dc;
pub const SMB_NTSTATUS_INVALID_DOMAIN_STATE:                                           u32 = 0xc00000dd;
pub const SMB_NTSTATUS_INVALID_DOMAIN_ROLE:                                            u32 = 0xc00000de;
pub const SMB_NTSTATUS_NO_SUCH_DOMAIN:                                                 u32 = 0xc00000df;
pub const SMB_NTSTATUS_DOMAIN_EXISTS:                                                  u32 = 0xc00000e0;
pub const SMB_NTSTATUS_DOMAIN_LIMIT_EXCEEDED:                                          u32 = 0xc00000e1;
pub const SMB_NTSTATUS_OPLOCK_NOT_GRANTED:                                             u32 = 0xc00000e2;
pub const SMB_NTSTATUS_INVALID_OPLOCK_PROTOCOL:                                        u32 = 0xc00000e3;
pub const SMB_NTSTATUS_INTERNAL_DB_CORRUPTION:                                         u32 = 0xc00000e4;
pub const SMB_NTSTATUS_INTERNAL_ERROR:                                                 u32 = 0xc00000e5;
pub const SMB_NTSTATUS_GENERIC_NOT_MAPPED:                                             u32 = 0xc00000e6;
pub const SMB_NTSTATUS_BAD_DESCRIPTOR_FORMAT:                                          u32 = 0xc00000e7;
pub const SMB_NTSTATUS_INVALID_USER_BUFFER:                                            u32 = 0xc00000e8;
pub const SMB_NTSTATUS_UNEXPECTED_IO_ERROR:                                            u32 = 0xc00000e9;
pub const SMB_NTSTATUS_UNEXPECTED_MM_CREATE_ERR:                                       u32 = 0xc00000ea;
pub const SMB_NTSTATUS_UNEXPECTED_MM_MAP_ERROR:                                        u32 = 0xc00000eb;
pub const SMB_NTSTATUS_UNEXPECTED_MM_EXTEND_ERR:                                       u32 = 0xc00000ec;
pub const SMB_NTSTATUS_NOT_LOGON_PROCESS:                                              u32 = 0xc00000ed;
pub const SMB_NTSTATUS_LOGON_SESSION_EXISTS:                                           u32 = 0xc00000ee;
pub const SMB_NTSTATUS_INVALID_PARAMETER_1:                                            u32 = 0xc00000ef;
pub const SMB_NTSTATUS_INVALID_PARAMETER_2:                                            u32 = 0xc00000f0;
pub const SMB_NTSTATUS_INVALID_PARAMETER_3:                                            u32 = 0xc00000f1;
pub const SMB_NTSTATUS_INVALID_PARAMETER_4:                                            u32 = 0xc00000f2;
pub const SMB_NTSTATUS_INVALID_PARAMETER_5:                                            u32 = 0xc00000f3;
pub const SMB_NTSTATUS_INVALID_PARAMETER_6:                                            u32 = 0xc00000f4;
pub const SMB_NTSTATUS_INVALID_PARAMETER_7:                                            u32 = 0xc00000f5;
pub const SMB_NTSTATUS_INVALID_PARAMETER_8:                                            u32 = 0xc00000f6;
pub const SMB_NTSTATUS_INVALID_PARAMETER_9:                                            u32 = 0xc00000f7;
pub const SMB_NTSTATUS_INVALID_PARAMETER_10:                                           u32 = 0xc00000f8;
pub const SMB_NTSTATUS_INVALID_PARAMETER_11:                                           u32 = 0xc00000f9;
pub const SMB_NTSTATUS_INVALID_PARAMETER_12:                                           u32 = 0xc00000fa;
pub const SMB_NTSTATUS_REDIRECTOR_NOT_STARTED:                                         u32 = 0xc00000fb;
pub const SMB_NTSTATUS_REDIRECTOR_STARTED:                                             u32 = 0xc00000fc;
pub const SMB_NTSTATUS_STACK_OVERFLOW:                                                 u32 = 0xc00000fd;
pub const SMB_NTSTATUS_NO_SUCH_PACKAGE:                                                u32 = 0xc00000fe;
pub const SMB_NTSTATUS_BAD_FUNCTION_TABLE:                                             u32 = 0xc00000ff;
pub const SMB_NTSTATUS_VARIABLE_NOT_FOUND:                                             u32 = 0xc0000100;
pub const SMB_NTSTATUS_DIRECTORY_NOT_EMPTY:                                            u32 = 0xc0000101;
pub const SMB_NTSTATUS_FILE_CORRUPT_ERROR:                                             u32 = 0xc0000102;
pub const SMB_NTSTATUS_NOT_A_DIRECTORY:                                                u32 = 0xc0000103;
pub const SMB_NTSTATUS_BAD_LOGON_SESSION_STATE:                                        u32 = 0xc0000104;
pub const SMB_NTSTATUS_LOGON_SESSION_COLLISION:                                        u32 = 0xc0000105;
pub const SMB_NTSTATUS_NAME_TOO_LONG:                                                  u32 = 0xc0000106;
pub const SMB_NTSTATUS_FILES_OPEN:                                                     u32 = 0xc0000107;
pub const SMB_NTSTATUS_CONNECTION_IN_USE:                                              u32 = 0xc0000108;
pub const SMB_NTSTATUS_MESSAGE_NOT_FOUND:                                              u32 = 0xc0000109;
pub const SMB_NTSTATUS_PROCESS_IS_TERMINATING:                                         u32 = 0xc000010a;
pub const SMB_NTSTATUS_INVALID_LOGON_TYPE:                                             u32 = 0xc000010b;
pub const SMB_NTSTATUS_NO_GUID_TRANSLATION:                                            u32 = 0xc000010c;
pub const SMB_NTSTATUS_CANNOT_IMPERSONATE:                                             u32 = 0xc000010d;
pub const SMB_NTSTATUS_IMAGE_ALREADY_LOADED:                                           u32 = 0xc000010e;
pub const SMB_NTSTATUS_NO_LDT:                                                         u32 = 0xc0000117;
pub const SMB_NTSTATUS_INVALID_LDT_SIZE:                                               u32 = 0xc0000118;
pub const SMB_NTSTATUS_INVALID_LDT_OFFSET:                                             u32 = 0xc0000119;
pub const SMB_NTSTATUS_INVALID_LDT_DESCRIPTOR:                                         u32 = 0xc000011a;
pub const SMB_NTSTATUS_INVALID_IMAGE_NE_FORMAT:                                        u32 = 0xc000011b;
pub const SMB_NTSTATUS_RXACT_INVALID_STATE:                                            u32 = 0xc000011c;
pub const SMB_NTSTATUS_RXACT_COMMIT_FAILURE:                                           u32 = 0xc000011d;
pub const SMB_NTSTATUS_MAPPED_FILE_SIZE_ZERO:                                          u32 = 0xc000011e;
pub const SMB_NTSTATUS_TOO_MANY_OPENED_FILES:                                          u32 = 0xc000011f;
pub const SMB_NTSTATUS_CANCELLED:                                                      u32 = 0xc0000120;
pub const SMB_NTSTATUS_CANNOT_DELETE:                                                  u32 = 0xc0000121;
pub const SMB_NTSTATUS_INVALID_COMPUTER_NAME:                                          u32 = 0xc0000122;
pub const SMB_NTSTATUS_FILE_DELETED:                                                   u32 = 0xc0000123;
pub const SMB_NTSTATUS_SPECIAL_ACCOUNT:                                                u32 = 0xc0000124;
pub const SMB_NTSTATUS_SPECIAL_GROUP:                                                  u32 = 0xc0000125;
pub const SMB_NTSTATUS_SPECIAL_USER:                                                   u32 = 0xc0000126;
pub const SMB_NTSTATUS_MEMBERS_PRIMARY_GROUP:                                          u32 = 0xc0000127;
pub const SMB_NTSTATUS_FILE_CLOSED:                                                    u32 = 0xc0000128;
pub const SMB_NTSTATUS_TOO_MANY_THREADS:                                               u32 = 0xc0000129;
pub const SMB_NTSTATUS_THREAD_NOT_IN_PROCESS:                                          u32 = 0xc000012a;
pub const SMB_NTSTATUS_TOKEN_ALREADY_IN_USE:                                           u32 = 0xc000012b;
pub const SMB_NTSTATUS_PAGEFILE_QUOTA_EXCEEDED:                                        u32 = 0xc000012c;
pub const SMB_NTSTATUS_COMMITMENT_LIMIT:                                               u32 = 0xc000012d;
pub const SMB_NTSTATUS_INVALID_IMAGE_LE_FORMAT:                                        u32 = 0xc000012e;
pub const SMB_NTSTATUS_INVALID_IMAGE_NOT_MZ:                                           u32 = 0xc000012f;
pub const SMB_NTSTATUS_INVALID_IMAGE_PROTECT:                                          u32 = 0xc0000130;
pub const SMB_NTSTATUS_INVALID_IMAGE_WIN_16:                                           u32 = 0xc0000131;
pub const SMB_NTSTATUS_LOGON_SERVER_CONFLICT:                                          u32 = 0xc0000132;
pub const SMB_NTSTATUS_TIME_DIFFERENCE_AT_DC:                                          u32 = 0xc0000133;
pub const SMB_NTSTATUS_SYNCHRONIZATION_REQUIRED:                                       u32 = 0xc0000134;
pub const SMB_NTSTATUS_DLL_NOT_FOUND:                                                  u32 = 0xc0000135;
pub const SMB_NTSTATUS_OPEN_FAILED:                                                    u32 = 0xc0000136;
pub const SMB_NTSTATUS_IO_PRIVILEGE_FAILED:                                            u32 = 0xc0000137;
pub const SMB_NTSTATUS_ORDINAL_NOT_FOUND:                                              u32 = 0xc0000138;
pub const SMB_NTSTATUS_ENTRYPOINT_NOT_FOUND:                                           u32 = 0xc0000139;
pub const SMB_NTSTATUS_CONTROL_C_EXIT:                                                 u32 = 0xc000013a;
pub const SMB_NTSTATUS_LOCAL_DISCONNECT:                                               u32 = 0xc000013b;
pub const SMB_NTSTATUS_REMOTE_DISCONNECT:                                              u32 = 0xc000013c;
pub const SMB_NTSTATUS_REMOTE_RESOURCES:                                               u32 = 0xc000013d;
pub const SMB_NTSTATUS_LINK_FAILED:                                                    u32 = 0xc000013e;
pub const SMB_NTSTATUS_LINK_TIMEOUT:                                                   u32 = 0xc000013f;
pub const SMB_NTSTATUS_INVALID_CONNECTION:                                             u32 = 0xc0000140;
pub const SMB_NTSTATUS_INVALID_ADDRESS:                                                u32 = 0xc0000141;
pub const SMB_NTSTATUS_DLL_INIT_FAILED:                                                u32 = 0xc0000142;
pub const SMB_NTSTATUS_MISSING_SYSTEMFILE:                                             u32 = 0xc0000143;
pub const SMB_NTSTATUS_UNHANDLED_EXCEPTION:                                            u32 = 0xc0000144;
pub const SMB_NTSTATUS_APP_INIT_FAILURE:                                               u32 = 0xc0000145;
pub const SMB_NTSTATUS_PAGEFILE_CREATE_FAILED:                                         u32 = 0xc0000146;
pub const SMB_NTSTATUS_NO_PAGEFILE:                                                    u32 = 0xc0000147;
pub const SMB_NTSTATUS_INVALID_LEVEL:                                                  u32 = 0xc0000148;
pub const SMB_NTSTATUS_WRONG_PASSWORD_CORE:                                            u32 = 0xc0000149;
pub const SMB_NTSTATUS_ILLEGAL_FLOAT_CONTEXT:                                          u32 = 0xc000014a;
pub const SMB_NTSTATUS_PIPE_BROKEN:                                                    u32 = 0xc000014b;
pub const SMB_NTSTATUS_REGISTRY_CORRUPT:                                               u32 = 0xc000014c;
pub const SMB_NTSTATUS_REGISTRY_IO_FAILED:                                             u32 = 0xc000014d;
pub const SMB_NTSTATUS_NO_EVENT_PAIR:                                                  u32 = 0xc000014e;
pub const SMB_NTSTATUS_UNRECOGNIZED_VOLUME:                                            u32 = 0xc000014f;
pub const SMB_NTSTATUS_SERIAL_NO_DEVICE_INITED:                                        u32 = 0xc0000150;
pub const SMB_NTSTATUS_NO_SUCH_ALIAS:                                                  u32 = 0xc0000151;
pub const SMB_NTSTATUS_MEMBER_NOT_IN_ALIAS:                                            u32 = 0xc0000152;
pub const SMB_NTSTATUS_MEMBER_IN_ALIAS:                                                u32 = 0xc0000153;
pub const SMB_NTSTATUS_ALIAS_EXISTS:                                                   u32 = 0xc0000154;
pub const SMB_NTSTATUS_LOGON_NOT_GRANTED:                                              u32 = 0xc0000155;
pub const SMB_NTSTATUS_TOO_MANY_SECRETS:                                               u32 = 0xc0000156;
pub const SMB_NTSTATUS_SECRET_TOO_LONG:                                                u32 = 0xc0000157;
pub const SMB_NTSTATUS_INTERNAL_DB_ERROR:                                              u32 = 0xc0000158;
pub const SMB_NTSTATUS_FULLSCREEN_MODE:                                                u32 = 0xc0000159;
pub const SMB_NTSTATUS_TOO_MANY_CONTEXT_IDS:                                           u32 = 0xc000015a;
pub const SMB_NTSTATUS_LOGON_TYPE_NOT_GRANTED:                                         u32 = 0xc000015b;
pub const SMB_NTSTATUS_NOT_REGISTRY_FILE:                                              u32 = 0xc000015c;
pub const SMB_NTSTATUS_NT_CROSS_ENCRYPTION_REQUIRED:                                   u32 = 0xc000015d;
pub const SMB_NTSTATUS_DOMAIN_CTRLR_CONFIG_ERROR:                                      u32 = 0xc000015e;
pub const SMB_NTSTATUS_FT_MISSING_MEMBER:                                              u32 = 0xc000015f;
pub const SMB_NTSTATUS_ILL_FORMED_SERVICE_ENTRY:                                       u32 = 0xc0000160;
pub const SMB_NTSTATUS_ILLEGAL_CHARACTER:                                              u32 = 0xc0000161;
pub const SMB_NTSTATUS_UNMAPPABLE_CHARACTER:                                           u32 = 0xc0000162;
pub const SMB_NTSTATUS_UNDEFINED_CHARACTER:                                            u32 = 0xc0000163;
pub const SMB_NTSTATUS_FLOPPY_VOLUME:                                                  u32 = 0xc0000164;
pub const SMB_NTSTATUS_FLOPPY_ID_MARK_NOT_FOUND:                                       u32 = 0xc0000165;
pub const SMB_NTSTATUS_FLOPPY_WRONG_CYLINDER:                                          u32 = 0xc0000166;
pub const SMB_NTSTATUS_FLOPPY_UNKNOWN_ERROR:                                           u32 = 0xc0000167;
pub const SMB_NTSTATUS_FLOPPY_BAD_REGISTERS:                                           u32 = 0xc0000168;
pub const SMB_NTSTATUS_DISK_RECALIBRATE_FAILED:                                        u32 = 0xc0000169;
pub const SMB_NTSTATUS_DISK_OPERATION_FAILED:                                          u32 = 0xc000016a;
pub const SMB_NTSTATUS_DISK_RESET_FAILED:                                              u32 = 0xc000016b;
pub const SMB_NTSTATUS_SHARED_IRQ_BUSY:                                                u32 = 0xc000016c;
pub const SMB_NTSTATUS_FT_ORPHANING:                                                   u32 = 0xc000016d;
pub const SMB_NTSTATUS_BIOS_FAILED_TO_CONNECT_INTERRUPT:                               u32 = 0xc000016e;
pub const SMB_NTSTATUS_PARTITION_FAILURE:                                              u32 = 0xc0000172;
pub const SMB_NTSTATUS_INVALID_BLOCK_LENGTH:                                           u32 = 0xc0000173;
pub const SMB_NTSTATUS_DEVICE_NOT_PARTITIONED:                                         u32 = 0xc0000174;
pub const SMB_NTSTATUS_UNABLE_TO_LOCK_MEDIA:                                           u32 = 0xc0000175;
pub const SMB_NTSTATUS_UNABLE_TO_UNLOAD_MEDIA:                                         u32 = 0xc0000176;
pub const SMB_NTSTATUS_EOM_OVERFLOW:                                                   u32 = 0xc0000177;
pub const SMB_NTSTATUS_NO_MEDIA:                                                       u32 = 0xc0000178;
pub const SMB_NTSTATUS_NO_SUCH_MEMBER:                                                 u32 = 0xc000017a;
pub const SMB_NTSTATUS_INVALID_MEMBER:                                                 u32 = 0xc000017b;
pub const SMB_NTSTATUS_KEY_DELETED:                                                    u32 = 0xc000017c;
pub const SMB_NTSTATUS_NO_LOG_SPACE:                                                   u32 = 0xc000017d;
pub const SMB_NTSTATUS_TOO_MANY_SIDS:                                                  u32 = 0xc000017e;
pub const SMB_NTSTATUS_LM_CROSS_ENCRYPTION_REQUIRED:                                   u32 = 0xc000017f;
pub const SMB_NTSTATUS_KEY_HAS_CHILDREN:                                               u32 = 0xc0000180;
pub const SMB_NTSTATUS_CHILD_MUST_BE_VOLATILE:                                         u32 = 0xc0000181;
pub const SMB_NTSTATUS_DEVICE_CONFIGURATION_ERROR:                                     u32 = 0xc0000182;
pub const SMB_NTSTATUS_DRIVER_INTERNAL_ERROR:                                          u32 = 0xc0000183;
pub const SMB_NTSTATUS_INVALID_DEVICE_STATE:                                           u32 = 0xc0000184;
pub const SMB_NTSTATUS_IO_DEVICE_ERROR:                                                u32 = 0xc0000185;
pub const SMB_NTSTATUS_DEVICE_PROTOCOL_ERROR:                                          u32 = 0xc0000186;
pub const SMB_NTSTATUS_BACKUP_CONTROLLER:                                              u32 = 0xc0000187;
pub const SMB_NTSTATUS_LOG_FILE_FULL:                                                  u32 = 0xc0000188;
pub const SMB_NTSTATUS_TOO_LATE:                                                       u32 = 0xc0000189;
pub const SMB_NTSTATUS_NO_TRUST_LSA_SECRET:                                            u32 = 0xc000018a;
pub const SMB_NTSTATUS_NO_TRUST_SAM_ACCOUNT:                                           u32 = 0xc000018b;
pub const SMB_NTSTATUS_TRUSTED_DOMAIN_FAILURE:                                         u32 = 0xc000018c;
pub const SMB_NTSTATUS_TRUSTED_RELATIONSHIP_FAILURE:                                   u32 = 0xc000018d;
pub const SMB_NTSTATUS_EVENTLOG_FILE_CORRUPT:                                          u32 = 0xc000018e;
pub const SMB_NTSTATUS_EVENTLOG_CANT_START:                                            u32 = 0xc000018f;
pub const SMB_NTSTATUS_TRUST_FAILURE:                                                  u32 = 0xc0000190;
pub const SMB_NTSTATUS_MUTANT_LIMIT_EXCEEDED:                                          u32 = 0xc0000191;
pub const SMB_NTSTATUS_NETLOGON_NOT_STARTED:                                           u32 = 0xc0000192;
pub const SMB_NTSTATUS_ACCOUNT_EXPIRED:                                                u32 = 0xc0000193;
pub const SMB_NTSTATUS_POSSIBLE_DEADLOCK:                                              u32 = 0xc0000194;
pub const SMB_NTSTATUS_NETWORK_CREDENTIAL_CONFLICT:                                    u32 = 0xc0000195;
pub const SMB_NTSTATUS_REMOTE_SESSION_LIMIT:                                           u32 = 0xc0000196;
pub const SMB_NTSTATUS_EVENTLOG_FILE_CHANGED:                                          u32 = 0xc0000197;
pub const SMB_NTSTATUS_NOLOGON_INTERDOMAIN_TRUST_ACCOUNT:                              u32 = 0xc0000198;
pub const SMB_NTSTATUS_NOLOGON_WORKSTATION_TRUST_ACCOUNT:                              u32 = 0xc0000199;
pub const SMB_NTSTATUS_NOLOGON_SERVER_TRUST_ACCOUNT:                                   u32 = 0xc000019a;
pub const SMB_NTSTATUS_DOMAIN_TRUST_INCONSISTENT:                                      u32 = 0xc000019b;
pub const SMB_NTSTATUS_FS_DRIVER_REQUIRED:                                             u32 = 0xc000019c;
pub const SMB_NTSTATUS_IMAGE_ALREADY_LOADED_AS_DLL:                                    u32 = 0xc000019d;
pub const SMB_NTSTATUS_INCOMPATIBLE_WITH_GLOBAL_SHORT_NAME_REGISTRY_SETTING:           u32 = 0xc000019e;
pub const SMB_NTSTATUS_SHORT_NAMES_NOT_ENABLED_ON_VOLUME:                              u32 = 0xc000019f;
pub const SMB_NTSTATUS_SECURITY_STREAM_IS_INCONSISTENT:                                u32 = 0xc00001a0;
pub const SMB_NTSTATUS_INVALID_LOCK_RANGE:                                             u32 = 0xc00001a1;
pub const SMB_NTSTATUS_INVALID_ACE_CONDITION:                                          u32 = 0xc00001a2;
pub const SMB_NTSTATUS_IMAGE_SUBSYSTEM_NOT_PRESENT:                                    u32 = 0xc00001a3;
pub const SMB_NTSTATUS_NOTIFICATION_GUID_ALREADY_DEFINED:                              u32 = 0xc00001a4;
pub const SMB_NTSTATUS_NETWORK_OPEN_RESTRICTION:                                       u32 = 0xc0000201;
pub const SMB_NTSTATUS_NO_USER_SESSION_KEY:                                            u32 = 0xc0000202;
pub const SMB_NTSTATUS_USER_SESSION_DELETED:                                           u32 = 0xc0000203;
pub const SMB_NTSTATUS_RESOURCE_LANG_NOT_FOUND:                                        u32 = 0xc0000204;
pub const SMB_NTSTATUS_INSUFF_SERVER_RESOURCES:                                        u32 = 0xc0000205;
pub const SMB_NTSTATUS_INVALID_BUFFER_SIZE:                                            u32 = 0xc0000206;
pub const SMB_NTSTATUS_INVALID_ADDRESS_COMPONENT:                                      u32 = 0xc0000207;
pub const SMB_NTSTATUS_INVALID_ADDRESS_WILDCARD:                                       u32 = 0xc0000208;
pub const SMB_NTSTATUS_TOO_MANY_ADDRESSES:                                             u32 = 0xc0000209;
pub const SMB_NTSTATUS_ADDRESS_ALREADY_EXISTS:                                         u32 = 0xc000020a;
pub const SMB_NTSTATUS_ADDRESS_CLOSED:                                                 u32 = 0xc000020b;
pub const SMB_NTSTATUS_CONNECTION_DISCONNECTED:                                        u32 = 0xc000020c;
pub const SMB_NTSTATUS_CONNECTION_RESET:                                               u32 = 0xc000020d;
pub const SMB_NTSTATUS_TOO_MANY_NODES:                                                 u32 = 0xc000020e;
pub const SMB_NTSTATUS_TRANSACTION_ABORTED:                                            u32 = 0xc000020f;
pub const SMB_NTSTATUS_TRANSACTION_TIMED_OUT:                                          u32 = 0xc0000210;
pub const SMB_NTSTATUS_TRANSACTION_NO_RELEASE:                                         u32 = 0xc0000211;
pub const SMB_NTSTATUS_TRANSACTION_NO_MATCH:                                           u32 = 0xc0000212;
pub const SMB_NTSTATUS_TRANSACTION_RESPONDED:                                          u32 = 0xc0000213;
pub const SMB_NTSTATUS_TRANSACTION_INVALID_ID:                                         u32 = 0xc0000214;
pub const SMB_NTSTATUS_TRANSACTION_INVALID_TYPE:                                       u32 = 0xc0000215;
pub const SMB_NTSTATUS_NOT_SERVER_SESSION:                                             u32 = 0xc0000216;
pub const SMB_NTSTATUS_NOT_CLIENT_SESSION:                                             u32 = 0xc0000217;
pub const SMB_NTSTATUS_CANNOT_LOAD_REGISTRY_FILE:                                      u32 = 0xc0000218;
pub const SMB_NTSTATUS_DEBUG_ATTACH_FAILED:                                            u32 = 0xc0000219;
pub const SMB_NTSTATUS_SYSTEM_PROCESS_TERMINATED:                                      u32 = 0xc000021a;
pub const SMB_NTSTATUS_DATA_NOT_ACCEPTED:                                              u32 = 0xc000021b;
pub const SMB_NTSTATUS_NO_BROWSER_SERVERS_FOUND:                                       u32 = 0xc000021c;
pub const SMB_NTSTATUS_VDM_HARD_ERROR:                                                 u32 = 0xc000021d;
pub const SMB_NTSTATUS_DRIVER_CANCEL_TIMEOUT:                                          u32 = 0xc000021e;
pub const SMB_NTSTATUS_REPLY_MESSAGE_MISMATCH:                                         u32 = 0xc000021f;
pub const SMB_NTSTATUS_MAPPED_ALIGNMENT:                                               u32 = 0xc0000220;
pub const SMB_NTSTATUS_IMAGE_CHECKSUM_MISMATCH:                                        u32 = 0xc0000221;
pub const SMB_NTSTATUS_LOST_WRITEBEHIND_DATA:                                          u32 = 0xc0000222;
pub const SMB_NTSTATUS_CLIENT_SERVER_PARAMETERS_INVALID:                               u32 = 0xc0000223;
pub const SMB_NTSTATUS_PASSWORD_MUST_CHANGE:                                           u32 = 0xc0000224;
pub const SMB_NTSTATUS_NOT_FOUND:                                                      u32 = 0xc0000225;
pub const SMB_NTSTATUS_NOT_TINY_STREAM:                                                u32 = 0xc0000226;
pub const SMB_NTSTATUS_RECOVERY_FAILURE:                                               u32 = 0xc0000227;
pub const SMB_NTSTATUS_STACK_OVERFLOW_READ:                                            u32 = 0xc0000228;
pub const SMB_NTSTATUS_FAIL_CHECK:                                                     u32 = 0xc0000229;
pub const SMB_NTSTATUS_DUPLICATE_OBJECTID:                                             u32 = 0xc000022a;
pub const SMB_NTSTATUS_OBJECTID_EXISTS:                                                u32 = 0xc000022b;
pub const SMB_NTSTATUS_CONVERT_TO_LARGE:                                               u32 = 0xc000022c;
pub const SMB_NTSTATUS_RETRY:                                                          u32 = 0xc000022d;
pub const SMB_NTSTATUS_FOUND_OUT_OF_SCOPE:                                             u32 = 0xc000022e;
pub const SMB_NTSTATUS_ALLOCATE_BUCKET:                                                u32 = 0xc000022f;
pub const SMB_NTSTATUS_PROPSET_NOT_FOUND:                                              u32 = 0xc0000230;
pub const SMB_NTSTATUS_MARSHALL_OVERFLOW:                                              u32 = 0xc0000231;
pub const SMB_NTSTATUS_INVALID_VARIANT:                                                u32 = 0xc0000232;
pub const SMB_NTSTATUS_DOMAIN_CONTROLLER_NOT_FOUND:                                    u32 = 0xc0000233;
pub const SMB_NTSTATUS_ACCOUNT_LOCKED_OUT:                                             u32 = 0xc0000234;
pub const SMB_NTSTATUS_HANDLE_NOT_CLOSABLE:                                            u32 = 0xc0000235;
pub const SMB_NTSTATUS_CONNECTION_REFUSED:                                             u32 = 0xc0000236;
pub const SMB_NTSTATUS_GRACEFUL_DISCONNECT:                                            u32 = 0xc0000237;
pub const SMB_NTSTATUS_ADDRESS_ALREADY_ASSOCIATED:                                     u32 = 0xc0000238;
pub const SMB_NTSTATUS_ADDRESS_NOT_ASSOCIATED:                                         u32 = 0xc0000239;
pub const SMB_NTSTATUS_CONNECTION_INVALID:                                             u32 = 0xc000023a;
pub const SMB_NTSTATUS_CONNECTION_ACTIVE:                                              u32 = 0xc000023b;
pub const SMB_NTSTATUS_NETWORK_UNREACHABLE:                                            u32 = 0xc000023c;
pub const SMB_NTSTATUS_HOST_UNREACHABLE:                                               u32 = 0xc000023d;
pub const SMB_NTSTATUS_PROTOCOL_UNREACHABLE:                                           u32 = 0xc000023e;
pub const SMB_NTSTATUS_PORT_UNREACHABLE:                                               u32 = 0xc000023f;
pub const SMB_NTSTATUS_REQUEST_ABORTED:                                                u32 = 0xc0000240;
pub const SMB_NTSTATUS_CONNECTION_ABORTED:                                             u32 = 0xc0000241;
pub const SMB_NTSTATUS_BAD_COMPRESSION_BUFFER:                                         u32 = 0xc0000242;
pub const SMB_NTSTATUS_USER_MAPPED_FILE:                                               u32 = 0xc0000243;
pub const SMB_NTSTATUS_AUDIT_FAILED:                                                   u32 = 0xc0000244;
pub const SMB_NTSTATUS_TIMER_RESOLUTION_NOT_SET:                                       u32 = 0xc0000245;
pub const SMB_NTSTATUS_CONNECTION_COUNT_LIMIT:                                         u32 = 0xc0000246;
pub const SMB_NTSTATUS_LOGIN_TIME_RESTRICTION:                                         u32 = 0xc0000247;
pub const SMB_NTSTATUS_LOGIN_WKSTA_RESTRICTION:                                        u32 = 0xc0000248;
pub const SMB_NTSTATUS_IMAGE_MP_UP_MISMATCH:                                           u32 = 0xc0000249;
pub const SMB_NTSTATUS_INSUFFICIENT_LOGON_INFO:                                        u32 = 0xc0000250;
pub const SMB_NTSTATUS_BAD_DLL_ENTRYPOINT:                                             u32 = 0xc0000251;
pub const SMB_NTSTATUS_BAD_SERVICE_ENTRYPOINT:                                         u32 = 0xc0000252;
pub const SMB_NTSTATUS_LPC_REPLY_LOST:                                                 u32 = 0xc0000253;
pub const SMB_NTSTATUS_IP_ADDRESS_CONFLICT1:                                           u32 = 0xc0000254;
pub const SMB_NTSTATUS_IP_ADDRESS_CONFLICT2:                                           u32 = 0xc0000255;
pub const SMB_NTSTATUS_REGISTRY_QUOTA_LIMIT:                                           u32 = 0xc0000256;
pub const SMB_NTSTATUS_PATH_NOT_COVERED:                                               u32 = 0xc0000257;
pub const SMB_NTSTATUS_NO_CALLBACK_ACTIVE:                                             u32 = 0xc0000258;
pub const SMB_NTSTATUS_LICENSE_QUOTA_EXCEEDED:                                         u32 = 0xc0000259;
pub const SMB_NTSTATUS_PWD_TOO_SHORT:                                                  u32 = 0xc000025a;
pub const SMB_NTSTATUS_PWD_TOO_RECENT:                                                 u32 = 0xc000025b;
pub const SMB_NTSTATUS_PWD_HISTORY_CONFLICT:                                           u32 = 0xc000025c;
pub const SMB_NTSTATUS_PLUGPLAY_NO_DEVICE:                                             u32 = 0xc000025e;
pub const SMB_NTSTATUS_UNSUPPORTED_COMPRESSION:                                        u32 = 0xc000025f;
pub const SMB_NTSTATUS_INVALID_HW_PROFILE:                                             u32 = 0xc0000260;
pub const SMB_NTSTATUS_INVALID_PLUGPLAY_DEVICE_PATH:                                   u32 = 0xc0000261;
pub const SMB_NTSTATUS_DRIVER_ORDINAL_NOT_FOUND:                                       u32 = 0xc0000262;
pub const SMB_NTSTATUS_DRIVER_ENTRYPOINT_NOT_FOUND:                                    u32 = 0xc0000263;
pub const SMB_NTSTATUS_RESOURCE_NOT_OWNED:                                             u32 = 0xc0000264;
pub const SMB_NTSTATUS_TOO_MANY_LINKS:                                                 u32 = 0xc0000265;
pub const SMB_NTSTATUS_QUOTA_LIST_INCONSISTENT:                                        u32 = 0xc0000266;
pub const SMB_NTSTATUS_FILE_IS_OFFLINE:                                                u32 = 0xc0000267;
pub const SMB_NTSTATUS_EVALUATION_EXPIRATION:                                          u32 = 0xc0000268;
pub const SMB_NTSTATUS_ILLEGAL_DLL_RELOCATION:                                         u32 = 0xc0000269;
pub const SMB_NTSTATUS_LICENSE_VIOLATION:                                              u32 = 0xc000026a;
pub const SMB_NTSTATUS_DLL_INIT_FAILED_LOGOFF:                                         u32 = 0xc000026b;
pub const SMB_NTSTATUS_DRIVER_UNABLE_TO_LOAD:                                          u32 = 0xc000026c;
pub const SMB_NTSTATUS_DFS_UNAVAILABLE:                                                u32 = 0xc000026d;
pub const SMB_NTSTATUS_VOLUME_DISMOUNTED:                                              u32 = 0xc000026e;
pub const SMB_NTSTATUS_WX86_INTERNAL_ERROR:                                            u32 = 0xc000026f;
pub const SMB_NTSTATUS_WX86_FLOAT_STACK_CHECK:                                         u32 = 0xc0000270;
pub const SMB_NTSTATUS_VALIDATE_CONTINUE:                                              u32 = 0xc0000271;
pub const SMB_NTSTATUS_NO_MATCH:                                                       u32 = 0xc0000272;
pub const SMB_NTSTATUS_NO_MORE_MATCHES:                                                u32 = 0xc0000273;
pub const SMB_NTSTATUS_NOT_A_REPARSE_POINT:                                            u32 = 0xc0000275;
pub const SMB_NTSTATUS_IO_REPARSE_TAG_INVALID:                                         u32 = 0xc0000276;
pub const SMB_NTSTATUS_IO_REPARSE_TAG_MISMATCH:                                        u32 = 0xc0000277;
pub const SMB_NTSTATUS_IO_REPARSE_DATA_INVALID:                                        u32 = 0xc0000278;
pub const SMB_NTSTATUS_IO_REPARSE_TAG_NOT_HANDLED:                                     u32 = 0xc0000279;
pub const SMB_NTSTATUS_REPARSE_POINT_NOT_RESOLVED:                                     u32 = 0xc0000280;
pub const SMB_NTSTATUS_DIRECTORY_IS_A_REPARSE_POINT:                                   u32 = 0xc0000281;
pub const SMB_NTSTATUS_RANGE_LIST_CONFLICT:                                            u32 = 0xc0000282;
pub const SMB_NTSTATUS_SOURCE_ELEMENT_EMPTY:                                           u32 = 0xc0000283;
pub const SMB_NTSTATUS_DESTINATION_ELEMENT_FULL:                                       u32 = 0xc0000284;
pub const SMB_NTSTATUS_ILLEGAL_ELEMENT_ADDRESS:                                        u32 = 0xc0000285;
pub const SMB_NTSTATUS_MAGAZINE_NOT_PRESENT:                                           u32 = 0xc0000286;
pub const SMB_NTSTATUS_REINITIALIZATION_NEEDED:                                        u32 = 0xc0000287;
pub const SMB_NTSTATUS_ENCRYPTION_FAILED:                                              u32 = 0xc000028a;
pub const SMB_NTSTATUS_DECRYPTION_FAILED:                                              u32 = 0xc000028b;
pub const SMB_NTSTATUS_RANGE_NOT_FOUND:                                                u32 = 0xc000028c;
pub const SMB_NTSTATUS_NO_RECOVERY_POLICY:                                             u32 = 0xc000028d;
pub const SMB_NTSTATUS_NO_EFS:                                                         u32 = 0xc000028e;
pub const SMB_NTSTATUS_WRONG_EFS:                                                      u32 = 0xc000028f;
pub const SMB_NTSTATUS_NO_USER_KEYS:                                                   u32 = 0xc0000290;
pub const SMB_NTSTATUS_FILE_NOT_ENCRYPTED:                                             u32 = 0xc0000291;
pub const SMB_NTSTATUS_NOT_EXPORT_FORMAT:                                              u32 = 0xc0000292;
pub const SMB_NTSTATUS_FILE_ENCRYPTED:                                                 u32 = 0xc0000293;
pub const SMB_NTSTATUS_WMI_GUID_NOT_FOUND:                                             u32 = 0xc0000295;
pub const SMB_NTSTATUS_WMI_INSTANCE_NOT_FOUND:                                         u32 = 0xc0000296;
pub const SMB_NTSTATUS_WMI_ITEMID_NOT_FOUND:                                           u32 = 0xc0000297;
pub const SMB_NTSTATUS_WMI_TRY_AGAIN:                                                  u32 = 0xc0000298;
pub const SMB_NTSTATUS_SHARED_POLICY:                                                  u32 = 0xc0000299;
pub const SMB_NTSTATUS_POLICY_OBJECT_NOT_FOUND:                                        u32 = 0xc000029a;
pub const SMB_NTSTATUS_POLICY_ONLY_IN_DS:                                              u32 = 0xc000029b;
pub const SMB_NTSTATUS_VOLUME_NOT_UPGRADED:                                            u32 = 0xc000029c;
pub const SMB_NTSTATUS_REMOTE_STORAGE_NOT_ACTIVE:                                      u32 = 0xc000029d;
pub const SMB_NTSTATUS_REMOTE_STORAGE_MEDIA_ERROR:                                     u32 = 0xc000029e;
pub const SMB_NTSTATUS_NO_TRACKING_SERVICE:                                            u32 = 0xc000029f;
pub const SMB_NTSTATUS_SERVER_SID_MISMATCH:                                            u32 = 0xc00002a0;
pub const SMB_NTSTATUS_DS_NO_ATTRIBUTE_OR_VALUE:                                       u32 = 0xc00002a1;
pub const SMB_NTSTATUS_DS_INVALID_ATTRIBUTE_SYNTAX:                                    u32 = 0xc00002a2;
pub const SMB_NTSTATUS_DS_ATTRIBUTE_TYPE_UNDEFINED:                                    u32 = 0xc00002a3;
pub const SMB_NTSTATUS_DS_ATTRIBUTE_OR_VALUE_EXISTS:                                   u32 = 0xc00002a4;
pub const SMB_NTSTATUS_DS_BUSY:                                                        u32 = 0xc00002a5;
pub const SMB_NTSTATUS_DS_UNAVAILABLE:                                                 u32 = 0xc00002a6;
pub const SMB_NTSTATUS_DS_NO_RIDS_ALLOCATED:                                           u32 = 0xc00002a7;
pub const SMB_NTSTATUS_DS_NO_MORE_RIDS:                                                u32 = 0xc00002a8;
pub const SMB_NTSTATUS_DS_INCORRECT_ROLE_OWNER:                                        u32 = 0xc00002a9;
pub const SMB_NTSTATUS_DS_RIDMGR_INIT_ERROR:                                           u32 = 0xc00002aa;
pub const SMB_NTSTATUS_DS_OBJ_CLASS_VIOLATION:                                         u32 = 0xc00002ab;
pub const SMB_NTSTATUS_DS_CANT_ON_NON_LEAF:                                            u32 = 0xc00002ac;
pub const SMB_NTSTATUS_DS_CANT_ON_RDN:                                                 u32 = 0xc00002ad;
pub const SMB_NTSTATUS_DS_CANT_MOD_OBJ_CLASS:                                          u32 = 0xc00002ae;
pub const SMB_NTSTATUS_DS_CROSS_DOM_MOVE_FAILED:                                       u32 = 0xc00002af;
pub const SMB_NTSTATUS_DS_GC_NOT_AVAILABLE:                                            u32 = 0xc00002b0;
pub const SMB_NTSTATUS_DIRECTORY_SERVICE_REQUIRED:                                     u32 = 0xc00002b1;
pub const SMB_NTSTATUS_REPARSE_ATTRIBUTE_CONFLICT:                                     u32 = 0xc00002b2;
pub const SMB_NTSTATUS_CANT_ENABLE_DENY_ONLY:                                          u32 = 0xc00002b3;
pub const SMB_NTSTATUS_FLOAT_MULTIPLE_FAULTS:                                          u32 = 0xc00002b4;
pub const SMB_NTSTATUS_FLOAT_MULTIPLE_TRAPS:                                           u32 = 0xc00002b5;
pub const SMB_NTSTATUS_DEVICE_REMOVED:                                                 u32 = 0xc00002b6;
pub const SMB_NTSTATUS_JOURNAL_DELETE_IN_PROGRESS:                                     u32 = 0xc00002b7;
pub const SMB_NTSTATUS_JOURNAL_NOT_ACTIVE:                                             u32 = 0xc00002b8;
pub const SMB_NTSTATUS_NOINTERFACE:                                                    u32 = 0xc00002b9;
pub const SMB_NTSTATUS_DS_ADMIN_LIMIT_EXCEEDED:                                        u32 = 0xc00002c1;
pub const SMB_NTSTATUS_DRIVER_FAILED_SLEEP:                                            u32 = 0xc00002c2;
pub const SMB_NTSTATUS_MUTUAL_AUTHENTICATION_FAILED:                                   u32 = 0xc00002c3;
pub const SMB_NTSTATUS_CORRUPT_SYSTEM_FILE:                                            u32 = 0xc00002c4;
pub const SMB_NTSTATUS_DATATYPE_MISALIGNMENT_ERROR:                                    u32 = 0xc00002c5;
pub const SMB_NTSTATUS_WMI_READ_ONLY:                                                  u32 = 0xc00002c6;
pub const SMB_NTSTATUS_WMI_SET_FAILURE:                                                u32 = 0xc00002c7;
pub const SMB_NTSTATUS_COMMITMENT_MINIMUM:                                             u32 = 0xc00002c8;
pub const SMB_NTSTATUS_REG_NAT_CONSUMPTION:                                            u32 = 0xc00002c9;
pub const SMB_NTSTATUS_TRANSPORT_FULL:                                                 u32 = 0xc00002ca;
pub const SMB_NTSTATUS_DS_SAM_INIT_FAILURE:                                            u32 = 0xc00002cb;
pub const SMB_NTSTATUS_ONLY_IF_CONNECTED:                                              u32 = 0xc00002cc;
pub const SMB_NTSTATUS_DS_SENSITIVE_GROUP_VIOLATION:                                   u32 = 0xc00002cd;
pub const SMB_NTSTATUS_PNP_RESTART_ENUMERATION:                                        u32 = 0xc00002ce;
pub const SMB_NTSTATUS_JOURNAL_ENTRY_DELETED:                                          u32 = 0xc00002cf;
pub const SMB_NTSTATUS_DS_CANT_MOD_PRIMARYGROUPID:                                     u32 = 0xc00002d0;
pub const SMB_NTSTATUS_SYSTEM_IMAGE_BAD_SIGNATURE:                                     u32 = 0xc00002d1;
pub const SMB_NTSTATUS_PNP_REBOOT_REQUIRED:                                            u32 = 0xc00002d2;
pub const SMB_NTSTATUS_POWER_STATE_INVALID:                                            u32 = 0xc00002d3;
pub const SMB_NTSTATUS_DS_INVALID_GROUP_TYPE:                                          u32 = 0xc00002d4;
pub const SMB_NTSTATUS_DS_NO_NEST_GLOBALGROUP_IN_MIXEDDOMAIN:                          u32 = 0xc00002d5;
pub const SMB_NTSTATUS_DS_NO_NEST_LOCALGROUP_IN_MIXEDDOMAIN:                           u32 = 0xc00002d6;
pub const SMB_NTSTATUS_DS_GLOBAL_CANT_HAVE_LOCAL_MEMBER:                               u32 = 0xc00002d7;
pub const SMB_NTSTATUS_DS_GLOBAL_CANT_HAVE_UNIVERSAL_MEMBER:                           u32 = 0xc00002d8;
pub const SMB_NTSTATUS_DS_UNIVERSAL_CANT_HAVE_LOCAL_MEMBER:                            u32 = 0xc00002d9;
pub const SMB_NTSTATUS_DS_GLOBAL_CANT_HAVE_CROSSDOMAIN_MEMBER:                         u32 = 0xc00002da;
pub const SMB_NTSTATUS_DS_LOCAL_CANT_HAVE_CROSSDOMAIN_LOCAL_MEMBER:                    u32 = 0xc00002db;
pub const SMB_NTSTATUS_DS_HAVE_PRIMARY_MEMBERS:                                        u32 = 0xc00002dc;
pub const SMB_NTSTATUS_WMI_NOT_SUPPORTED:                                              u32 = 0xc00002dd;
pub const SMB_NTSTATUS_INSUFFICIENT_POWER:                                             u32 = 0xc00002de;
pub const SMB_NTSTATUS_SAM_NEED_BOOTKEY_PASSWORD:                                      u32 = 0xc00002df;
pub const SMB_NTSTATUS_SAM_NEED_BOOTKEY_FLOPPY:                                        u32 = 0xc00002e0;
pub const SMB_NTSTATUS_DS_CANT_START:                                                  u32 = 0xc00002e1;
pub const SMB_NTSTATUS_DS_INIT_FAILURE:                                                u32 = 0xc00002e2;
pub const SMB_NTSTATUS_SAM_INIT_FAILURE:                                               u32 = 0xc00002e3;
pub const SMB_NTSTATUS_DS_GC_REQUIRED:                                                 u32 = 0xc00002e4;
pub const SMB_NTSTATUS_DS_LOCAL_MEMBER_OF_LOCAL_ONLY:                                  u32 = 0xc00002e5;
pub const SMB_NTSTATUS_DS_NO_FPO_IN_UNIVERSAL_GROUPS:                                  u32 = 0xc00002e6;
pub const SMB_NTSTATUS_DS_MACHINE_ACCOUNT_QUOTA_EXCEEDED:                              u32 = 0xc00002e7;
pub const SMB_NTSTATUS_CURRENT_DOMAIN_NOT_ALLOWED:                                     u32 = 0xc00002e9;
pub const SMB_NTSTATUS_CANNOT_MAKE:                                                    u32 = 0xc00002ea;
pub const SMB_NTSTATUS_SYSTEM_SHUTDOWN:                                                u32 = 0xc00002eb;
pub const SMB_NTSTATUS_DS_INIT_FAILURE_CONSOLE:                                        u32 = 0xc00002ec;
pub const SMB_NTSTATUS_DS_SAM_INIT_FAILURE_CONSOLE:                                    u32 = 0xc00002ed;
pub const SMB_NTSTATUS_UNFINISHED_CONTEXT_DELETED:                                     u32 = 0xc00002ee;
pub const SMB_NTSTATUS_NO_TGT_REPLY:                                                   u32 = 0xc00002ef;
pub const SMB_NTSTATUS_OBJECTID_NOT_FOUND:                                             u32 = 0xc00002f0;
pub const SMB_NTSTATUS_NO_IP_ADDRESSES:                                                u32 = 0xc00002f1;
pub const SMB_NTSTATUS_WRONG_CREDENTIAL_HANDLE:                                        u32 = 0xc00002f2;
pub const SMB_NTSTATUS_CRYPTO_SYSTEM_INVALID:                                          u32 = 0xc00002f3;
pub const SMB_NTSTATUS_MAX_REFERRALS_EXCEEDED:                                         u32 = 0xc00002f4;
pub const SMB_NTSTATUS_MUST_BE_KDC:                                                    u32 = 0xc00002f5;
pub const SMB_NTSTATUS_STRONG_CRYPTO_NOT_SUPPORTED:                                    u32 = 0xc00002f6;
pub const SMB_NTSTATUS_TOO_MANY_PRINCIPALS:                                            u32 = 0xc00002f7;
pub const SMB_NTSTATUS_NO_PA_DATA:                                                     u32 = 0xc00002f8;
pub const SMB_NTSTATUS_PKINIT_NAME_MISMATCH:                                           u32 = 0xc00002f9;
pub const SMB_NTSTATUS_SMARTCARD_LOGON_REQUIRED:                                       u32 = 0xc00002fa;
pub const SMB_NTSTATUS_KDC_INVALID_REQUEST:                                            u32 = 0xc00002fb;
pub const SMB_NTSTATUS_KDC_UNABLE_TO_REFER:                                            u32 = 0xc00002fc;
pub const SMB_NTSTATUS_KDC_UNKNOWN_ETYPE:                                              u32 = 0xc00002fd;
pub const SMB_NTSTATUS_SHUTDOWN_IN_PROGRESS:                                           u32 = 0xc00002fe;
pub const SMB_NTSTATUS_SERVER_SHUTDOWN_IN_PROGRESS:                                    u32 = 0xc00002ff;
pub const SMB_NTSTATUS_NOT_SUPPORTED_ON_SBS:                                           u32 = 0xc0000300;
pub const SMB_NTSTATUS_WMI_GUID_DISCONNECTED:                                          u32 = 0xc0000301;
pub const SMB_NTSTATUS_WMI_ALREADY_DISABLED:                                           u32 = 0xc0000302;
pub const SMB_NTSTATUS_WMI_ALREADY_ENABLED:                                            u32 = 0xc0000303;
pub const SMB_NTSTATUS_MFT_TOO_FRAGMENTED:                                             u32 = 0xc0000304;
pub const SMB_NTSTATUS_COPY_PROTECTION_FAILURE:                                        u32 = 0xc0000305;
pub const SMB_NTSTATUS_CSS_AUTHENTICATION_FAILURE:                                     u32 = 0xc0000306;
pub const SMB_NTSTATUS_CSS_KEY_NOT_PRESENT:                                            u32 = 0xc0000307;
pub const SMB_NTSTATUS_CSS_KEY_NOT_ESTABLISHED:                                        u32 = 0xc0000308;
pub const SMB_NTSTATUS_CSS_SCRAMBLED_SECTOR:                                           u32 = 0xc0000309;
pub const SMB_NTSTATUS_CSS_REGION_MISMATCH:                                            u32 = 0xc000030a;
pub const SMB_NTSTATUS_CSS_RESETS_EXHAUSTED:                                           u32 = 0xc000030b;
pub const SMB_NTSTATUS_PKINIT_FAILURE:                                                 u32 = 0xc0000320;
pub const SMB_NTSTATUS_SMARTCARD_SUBSYSTEM_FAILURE:                                    u32 = 0xc0000321;
pub const SMB_NTSTATUS_NO_KERB_KEY:                                                    u32 = 0xc0000322;
pub const SMB_NTSTATUS_HOST_DOWN:                                                      u32 = 0xc0000350;
pub const SMB_NTSTATUS_UNSUPPORTED_PREAUTH:                                            u32 = 0xc0000351;
pub const SMB_NTSTATUS_EFS_ALG_BLOB_TOO_BIG:                                           u32 = 0xc0000352;
pub const SMB_NTSTATUS_PORT_NOT_SET:                                                   u32 = 0xc0000353;
pub const SMB_NTSTATUS_DEBUGGER_INACTIVE:                                              u32 = 0xc0000354;
pub const SMB_NTSTATUS_DS_VERSION_CHECK_FAILURE:                                       u32 = 0xc0000355;
pub const SMB_NTSTATUS_AUDITING_DISABLED:                                              u32 = 0xc0000356;
pub const SMB_NTSTATUS_PRENT4_MACHINE_ACCOUNT:                                         u32 = 0xc0000357;
pub const SMB_NTSTATUS_DS_AG_CANT_HAVE_UNIVERSAL_MEMBER:                               u32 = 0xc0000358;
pub const SMB_NTSTATUS_INVALID_IMAGE_WIN_32:                                           u32 = 0xc0000359;
pub const SMB_NTSTATUS_INVALID_IMAGE_WIN_64:                                           u32 = 0xc000035a;
pub const SMB_NTSTATUS_BAD_BINDINGS:                                                   u32 = 0xc000035b;
pub const SMB_NTSTATUS_NETWORK_SESSION_EXPIRED:                                        u32 = 0xc000035c;
pub const SMB_NTSTATUS_APPHELP_BLOCK:                                                  u32 = 0xc000035d;
pub const SMB_NTSTATUS_ALL_SIDS_FILTERED:                                              u32 = 0xc000035e;
pub const SMB_NTSTATUS_NOT_SAFE_MODE_DRIVER:                                           u32 = 0xc000035f;
pub const SMB_NTSTATUS_ACCESS_DISABLED_BY_POLICY_DEFAULT:                              u32 = 0xc0000361;
pub const SMB_NTSTATUS_ACCESS_DISABLED_BY_POLICY_PATH:                                 u32 = 0xc0000362;
pub const SMB_NTSTATUS_ACCESS_DISABLED_BY_POLICY_PUBLISHER:                            u32 = 0xc0000363;
pub const SMB_NTSTATUS_ACCESS_DISABLED_BY_POLICY_OTHER:                                u32 = 0xc0000364;
pub const SMB_NTSTATUS_FAILED_DRIVER_ENTRY:                                            u32 = 0xc0000365;
pub const SMB_NTSTATUS_DEVICE_ENUMERATION_ERROR:                                       u32 = 0xc0000366;
pub const SMB_NTSTATUS_MOUNT_POINT_NOT_RESOLVED:                                       u32 = 0xc0000368;
pub const SMB_NTSTATUS_INVALID_DEVICE_OBJECT_PARAMETER:                                u32 = 0xc0000369;
pub const SMB_NTSTATUS_MCA_OCCURED:                                                    u32 = 0xc000036a;
pub const SMB_NTSTATUS_DRIVER_BLOCKED_CRITICAL:                                        u32 = 0xc000036b;
pub const SMB_NTSTATUS_DRIVER_BLOCKED:                                                 u32 = 0xc000036c;
pub const SMB_NTSTATUS_DRIVER_DATABASE_ERROR:                                          u32 = 0xc000036d;
pub const SMB_NTSTATUS_SYSTEM_HIVE_TOO_LARGE:                                          u32 = 0xc000036e;
pub const SMB_NTSTATUS_INVALID_IMPORT_OF_NON_DLL:                                      u32 = 0xc000036f;
pub const SMB_NTSTATUS_NO_SECRETS:                                                     u32 = 0xc0000371;
pub const SMB_NTSTATUS_ACCESS_DISABLED_NO_SAFER_UI_BY_POLICY:                          u32 = 0xc0000372;
pub const SMB_NTSTATUS_FAILED_STACK_SWITCH:                                            u32 = 0xc0000373;
pub const SMB_NTSTATUS_HEAP_CORRUPTION:                                                u32 = 0xc0000374;
pub const SMB_NTSTATUS_SMARTCARD_WRONG_PIN:                                            u32 = 0xc0000380;
pub const SMB_NTSTATUS_SMARTCARD_CARD_BLOCKED:                                         u32 = 0xc0000381;
pub const SMB_NTSTATUS_SMARTCARD_CARD_NOT_AUTHENTICATED:                               u32 = 0xc0000382;
pub const SMB_NTSTATUS_SMARTCARD_NO_CARD:                                              u32 = 0xc0000383;
pub const SMB_NTSTATUS_SMARTCARD_NO_KEY_CONTAINER:                                     u32 = 0xc0000384;
pub const SMB_NTSTATUS_SMARTCARD_NO_CERTIFICATE:                                       u32 = 0xc0000385;
pub const SMB_NTSTATUS_SMARTCARD_NO_KEYSET:                                            u32 = 0xc0000386;
pub const SMB_NTSTATUS_SMARTCARD_IO_ERROR:                                             u32 = 0xc0000387;
pub const SMB_NTSTATUS_DOWNGRADE_DETECTED:                                             u32 = 0xc0000388;
pub const SMB_NTSTATUS_SMARTCARD_CERT_REVOKED:                                         u32 = 0xc0000389;
pub const SMB_NTSTATUS_ISSUING_CA_UNTRUSTED:                                           u32 = 0xc000038a;
pub const SMB_NTSTATUS_REVOCATION_OFFLINE_C:                                           u32 = 0xc000038b;
pub const SMB_NTSTATUS_PKINIT_CLIENT_FAILURE:                                          u32 = 0xc000038c;
pub const SMB_NTSTATUS_SMARTCARD_CERT_EXPIRED:                                         u32 = 0xc000038d;
pub const SMB_NTSTATUS_DRIVER_FAILED_PRIOR_UNLOAD:                                     u32 = 0xc000038e;
pub const SMB_NTSTATUS_SMARTCARD_SILENT_CONTEXT:                                       u32 = 0xc000038f;
pub const SMB_NTSTATUS_PER_USER_TRUST_QUOTA_EXCEEDED:                                  u32 = 0xc0000401;
pub const SMB_NTSTATUS_ALL_USER_TRUST_QUOTA_EXCEEDED:                                  u32 = 0xc0000402;
pub const SMB_NTSTATUS_USER_DELETE_TRUST_QUOTA_EXCEEDED:                               u32 = 0xc0000403;
pub const SMB_NTSTATUS_DS_NAME_NOT_UNIQUE:                                             u32 = 0xc0000404;
pub const SMB_NTSTATUS_DS_DUPLICATE_ID_FOUND:                                          u32 = 0xc0000405;
pub const SMB_NTSTATUS_DS_GROUP_CONVERSION_ERROR:                                      u32 = 0xc0000406;
pub const SMB_NTSTATUS_VOLSNAP_PREPARE_HIBERNATE:                                      u32 = 0xc0000407;
pub const SMB_NTSTATUS_USER2USER_REQUIRED:                                             u32 = 0xc0000408;
pub const SMB_NTSTATUS_STACK_BUFFER_OVERRUN:                                           u32 = 0xc0000409;
pub const SMB_NTSTATUS_NO_S4U_PROT_SUPPORT:                                            u32 = 0xc000040a;
pub const SMB_NTSTATUS_CROSSREALM_DELEGATION_FAILURE:                                  u32 = 0xc000040b;
pub const SMB_NTSTATUS_REVOCATION_OFFLINE_KDC:                                         u32 = 0xc000040c;
pub const SMB_NTSTATUS_ISSUING_CA_UNTRUSTED_KDC:                                       u32 = 0xc000040d;
pub const SMB_NTSTATUS_KDC_CERT_EXPIRED:                                               u32 = 0xc000040e;
pub const SMB_NTSTATUS_KDC_CERT_REVOKED:                                               u32 = 0xc000040f;
pub const SMB_NTSTATUS_PARAMETER_QUOTA_EXCEEDED:                                       u32 = 0xc0000410;
pub const SMB_NTSTATUS_HIBERNATION_FAILURE:                                            u32 = 0xc0000411;
pub const SMB_NTSTATUS_DELAY_LOAD_FAILED:                                              u32 = 0xc0000412;
pub const SMB_NTSTATUS_AUTHENTICATION_FIREWALL_FAILED:                                 u32 = 0xc0000413;
pub const SMB_NTSTATUS_VDM_DISALLOWED:                                                 u32 = 0xc0000414;
pub const SMB_NTSTATUS_HUNG_DISPLAY_DRIVER_THREAD:                                     u32 = 0xc0000415;
pub const SMB_NTSTATUS_INSUFFICIENT_RESOURCE_FOR_SPECIFIED_SHARED_SECTION_SIZE:        u32 = 0xc0000416;
pub const SMB_NTSTATUS_INVALID_CRUNTIME_PARAMETER:                                     u32 = 0xc0000417;
pub const SMB_NTSTATUS_NTLM_BLOCKED:                                                   u32 = 0xc0000418;
pub const SMB_NTSTATUS_DS_SRC_SID_EXISTS_IN_FOREST:                                    u32 = 0xc0000419;
pub const SMB_NTSTATUS_DS_DOMAIN_NAME_EXISTS_IN_FOREST:                                u32 = 0xc000041a;
pub const SMB_NTSTATUS_DS_FLAT_NAME_EXISTS_IN_FOREST:                                  u32 = 0xc000041b;
pub const SMB_NTSTATUS_INVALID_USER_PRINCIPAL_NAME:                                    u32 = 0xc000041c;
pub const SMB_NTSTATUS_ASSERTION_FAILURE:                                              u32 = 0xc0000420;
pub const SMB_NTSTATUS_VERIFIER_STOP:                                                  u32 = 0xc0000421;
pub const SMB_NTSTATUS_CALLBACK_POP_STACK:                                             u32 = 0xc0000423;
pub const SMB_NTSTATUS_INCOMPATIBLE_DRIVER_BLOCKED:                                    u32 = 0xc0000424;
pub const SMB_NTSTATUS_HIVE_UNLOADED:                                                  u32 = 0xc0000425;
pub const SMB_NTSTATUS_COMPRESSION_DISABLED:                                           u32 = 0xc0000426;
pub const SMB_NTSTATUS_FILE_SYSTEM_LIMITATION:                                         u32 = 0xc0000427;
pub const SMB_NTSTATUS_INVALID_IMAGE_HASH:                                             u32 = 0xc0000428;
pub const SMB_NTSTATUS_NOT_CAPABLE:                                                    u32 = 0xc0000429;
pub const SMB_NTSTATUS_REQUEST_OUT_OF_SEQUENCE:                                        u32 = 0xc000042a;
pub const SMB_NTSTATUS_IMPLEMENTATION_LIMIT:                                           u32 = 0xc000042b;
pub const SMB_NTSTATUS_ELEVATION_REQUIRED:                                             u32 = 0xc000042c;
pub const SMB_NTSTATUS_NO_SECURITY_CONTEXT:                                            u32 = 0xc000042d;
pub const SMB_NTSTATUS_PKU2U_CERT_FAILURE:                                             u32 = 0xc000042e;
pub const SMB_NTSTATUS_BEYOND_VDL:                                                     u32 = 0xc0000432;
pub const SMB_NTSTATUS_ENCOUNTERED_WRITE_IN_PROGRESS:                                  u32 = 0xc0000433;
pub const SMB_NTSTATUS_PTE_CHANGED:                                                    u32 = 0xc0000434;
pub const SMB_NTSTATUS_PURGE_FAILED:                                                   u32 = 0xc0000435;
pub const SMB_NTSTATUS_CRED_REQUIRES_CONFIRMATION:                                     u32 = 0xc0000440;
pub const SMB_NTSTATUS_CS_ENCRYPTION_INVALID_SERVER_RESPONSE:                          u32 = 0xc0000441;
pub const SMB_NTSTATUS_CS_ENCRYPTION_UNSUPPORTED_SERVER:                               u32 = 0xc0000442;
pub const SMB_NTSTATUS_CS_ENCRYPTION_EXISTING_ENCRYPTED_FILE:                          u32 = 0xc0000443;
pub const SMB_NTSTATUS_CS_ENCRYPTION_NEW_ENCRYPTED_FILE:                               u32 = 0xc0000444;
pub const SMB_NTSTATUS_CS_ENCRYPTION_FILE_NOT_CSE:                                     u32 = 0xc0000445;
pub const SMB_NTSTATUS_INVALID_LABEL:                                                  u32 = 0xc0000446;
pub const SMB_NTSTATUS_DRIVER_PROCESS_TERMINATED:                                      u32 = 0xc0000450;
pub const SMB_NTSTATUS_AMBIGUOUS_SYSTEM_DEVICE:                                        u32 = 0xc0000451;
pub const SMB_NTSTATUS_SYSTEM_DEVICE_NOT_FOUND:                                        u32 = 0xc0000452;
pub const SMB_NTSTATUS_RESTART_BOOT_APPLICATION:                                       u32 = 0xc0000453;
pub const SMB_NTSTATUS_INSUFFICIENT_NVRAM_RESOURCES:                                   u32 = 0xc0000454;
pub const SMB_NTSTATUS_NO_RANGES_PROCESSED:                                            u32 = 0xc0000460;
pub const SMB_NTSTATUS_DEVICE_FEATURE_NOT_SUPPORTED:                                   u32 = 0xc0000463;
pub const SMB_NTSTATUS_DEVICE_UNREACHABLE:                                             u32 = 0xc0000464;
pub const SMB_NTSTATUS_INVALID_TOKEN:                                                  u32 = 0xc0000465;
pub const SMB_NTSTATUS_SERVER_UNAVAILABLE:                                             u32 = 0xc0000466;
pub const SMB_NTSTATUS_INVALID_TASK_NAME:                                              u32 = 0xc0000500;
pub const SMB_NTSTATUS_INVALID_TASK_INDEX:                                             u32 = 0xc0000501;
pub const SMB_NTSTATUS_THREAD_ALREADY_IN_TASK:                                         u32 = 0xc0000502;
pub const SMB_NTSTATUS_CALLBACK_BYPASS:                                                u32 = 0xc0000503;
pub const SMB_NTSTATUS_FAIL_FAST_EXCEPTION:                                            u32 = 0xc0000602;
pub const SMB_NTSTATUS_IMAGE_CERT_REVOKED:                                             u32 = 0xc0000603;
pub const SMB_NTSTATUS_PORT_CLOSED:                                                    u32 = 0xc0000700;
pub const SMB_NTSTATUS_MESSAGE_LOST:                                                   u32 = 0xc0000701;
pub const SMB_NTSTATUS_INVALID_MESSAGE:                                                u32 = 0xc0000702;
pub const SMB_NTSTATUS_REQUEST_CANCELED:                                               u32 = 0xc0000703;
pub const SMB_NTSTATUS_RECURSIVE_DISPATCH:                                             u32 = 0xc0000704;
pub const SMB_NTSTATUS_LPC_RECEIVE_BUFFER_EXPECTED:                                    u32 = 0xc0000705;
pub const SMB_NTSTATUS_LPC_INVALID_CONNECTION_USAGE:                                   u32 = 0xc0000706;
pub const SMB_NTSTATUS_LPC_REQUESTS_NOT_ALLOWED:                                       u32 = 0xc0000707;
pub const SMB_NTSTATUS_RESOURCE_IN_USE:                                                u32 = 0xc0000708;
pub const SMB_NTSTATUS_HARDWARE_MEMORY_ERROR:                                          u32 = 0xc0000709;
pub const SMB_NTSTATUS_THREADPOOL_HANDLE_EXCEPTION:                                    u32 = 0xc000070a;
pub const SMB_NTSTATUS_THREADPOOL_SET_EVENT_ON_COMPLETION_FAILED:                      u32 = 0xc000070b;
pub const SMB_NTSTATUS_THREADPOOL_RELEASE_SEMAPHORE_ON_COMPLETION_FAILED:              u32 = 0xc000070c;
pub const SMB_NTSTATUS_THREADPOOL_RELEASE_MUTEX_ON_COMPLETION_FAILED:                  u32 = 0xc000070d;
pub const SMB_NTSTATUS_THREADPOOL_FREE_LIBRARY_ON_COMPLETION_FAILED:                   u32 = 0xc000070e;
pub const SMB_NTSTATUS_THREADPOOL_RELEASED_DURING_OPERATION:                           u32 = 0xc000070f;
pub const SMB_NTSTATUS_CALLBACK_RETURNED_WHILE_IMPERSONATING:                          u32 = 0xc0000710;
pub const SMB_NTSTATUS_APC_RETURNED_WHILE_IMPERSONATING:                               u32 = 0xc0000711;
pub const SMB_NTSTATUS_PROCESS_IS_PROTECTED:                                           u32 = 0xc0000712;
pub const SMB_NTSTATUS_MCA_EXCEPTION:                                                  u32 = 0xc0000713;
pub const SMB_NTSTATUS_CERTIFICATE_MAPPING_NOT_UNIQUE:                                 u32 = 0xc0000714;
pub const SMB_NTSTATUS_SYMLINK_CLASS_DISABLED:                                         u32 = 0xc0000715;
pub const SMB_NTSTATUS_INVALID_IDN_NORMALIZATION:                                      u32 = 0xc0000716;
pub const SMB_NTSTATUS_NO_UNICODE_TRANSLATION:                                         u32 = 0xc0000717;
pub const SMB_NTSTATUS_ALREADY_REGISTERED:                                             u32 = 0xc0000718;
pub const SMB_NTSTATUS_CONTEXT_MISMATCH:                                               u32 = 0xc0000719;
pub const SMB_NTSTATUS_PORT_ALREADY_HAS_COMPLETION_LIST:                               u32 = 0xc000071a;
pub const SMB_NTSTATUS_CALLBACK_RETURNED_THREAD_PRIORITY:                              u32 = 0xc000071b;
pub const SMB_NTSTATUS_INVALID_THREAD:                                                 u32 = 0xc000071c;
pub const SMB_NTSTATUS_CALLBACK_RETURNED_TRANSACTION:                                  u32 = 0xc000071d;
pub const SMB_NTSTATUS_CALLBACK_RETURNED_LDR_LOCK:                                     u32 = 0xc000071e;
pub const SMB_NTSTATUS_CALLBACK_RETURNED_LANG:                                         u32 = 0xc000071f;
pub const SMB_NTSTATUS_CALLBACK_RETURNED_PRI_BACK:                                     u32 = 0xc0000720;
pub const SMB_NTSTATUS_DISK_REPAIR_DISABLED:                                           u32 = 0xc0000800;
pub const SMB_NTSTATUS_DS_DOMAIN_RENAME_IN_PROGRESS:                                   u32 = 0xc0000801;
pub const SMB_NTSTATUS_DISK_QUOTA_EXCEEDED:                                            u32 = 0xc0000802;
pub const SMB_NTSTATUS_CONTENT_BLOCKED:                                                u32 = 0xc0000804;
pub const SMB_NTSTATUS_BAD_CLUSTERS:                                                   u32 = 0xc0000805;
pub const SMB_NTSTATUS_VOLUME_DIRTY:                                                   u32 = 0xc0000806;
pub const SMB_NTSTATUS_FILE_CHECKED_OUT:                                               u32 = 0xc0000901;
pub const SMB_NTSTATUS_CHECKOUT_REQUIRED:                                              u32 = 0xc0000902;
pub const SMB_NTSTATUS_BAD_FILE_TYPE:                                                  u32 = 0xc0000903;
pub const SMB_NTSTATUS_FILE_TOO_LARGE:                                                 u32 = 0xc0000904;
pub const SMB_NTSTATUS_FORMS_AUTH_REQUIRED:                                            u32 = 0xc0000905;
pub const SMB_NTSTATUS_VIRUS_INFECTED:                                                 u32 = 0xc0000906;
pub const SMB_NTSTATUS_VIRUS_DELETED:                                                  u32 = 0xc0000907;
pub const SMB_NTSTATUS_BAD_MCFG_TABLE:                                                 u32 = 0xc0000908;
pub const SMB_NTSTATUS_CANNOT_BREAK_OPLOCK:                                            u32 = 0xc0000909;
pub const SMB_NTSTATUS_WOW_ASSERTION:                                                  u32 = 0xc0009898;
pub const SMB_NTSTATUS_INVALID_SIGNATURE:                                              u32 = 0xc000a000;
pub const SMB_NTSTATUS_HMAC_NOT_SUPPORTED:                                             u32 = 0xc000a001;
pub const SMB_NTSTATUS_IPSEC_QUEUE_OVERFLOW:                                           u32 = 0xc000a010;
pub const SMB_NTSTATUS_ND_QUEUE_OVERFLOW:                                              u32 = 0xc000a011;
pub const SMB_NTSTATUS_HOPLIMIT_EXCEEDED:                                              u32 = 0xc000a012;
pub const SMB_NTSTATUS_PROTOCOL_NOT_SUPPORTED:                                         u32 = 0xc000a013;
pub const SMB_NTSTATUS_LOST_WRITEBEHIND_DATA_NETWORK_DISCONNECTED:                     u32 = 0xc000a080;
pub const SMB_NTSTATUS_LOST_WRITEBEHIND_DATA_NETWORK_SERVER_ERROR:                     u32 = 0xc000a081;
pub const SMB_NTSTATUS_LOST_WRITEBEHIND_DATA_LOCAL_DISK_ERROR:                         u32 = 0xc000a082;
pub const SMB_NTSTATUS_XML_PARSE_ERROR:                                                u32 = 0xc000a083;
pub const SMB_NTSTATUS_XMLDSIG_ERROR:                                                  u32 = 0xc000a084;
pub const SMB_NTSTATUS_WRONG_COMPARTMENT:                                              u32 = 0xc000a085;
pub const SMB_NTSTATUS_AUTHIP_FAILURE:                                                 u32 = 0xc000a086;
pub const SMB_NTSTATUS_DS_OID_MAPPED_GROUP_CANT_HAVE_MEMBERS:                          u32 = 0xc000a087;
pub const SMB_NTSTATUS_DS_OID_NOT_FOUND:                                               u32 = 0xc000a088;
pub const SMB_NTSTATUS_HASH_NOT_SUPPORTED:                                             u32 = 0xc000a100;
pub const SMB_NTSTATUS_HASH_NOT_PRESENT:                                               u32 = 0xc000a101;
pub const SMB_NTSTATUS_OFFLOAD_READ_FLT_NOT_SUPPORTED:                                 u32 = 0xc000a2a1;
pub const SMB_NTSTATUS_OFFLOAD_WRITE_FLT_NOT_SUPPORTED:                                u32 = 0xc000a2a2;
pub const SMB_NTSTATUS_OFFLOAD_READ_FILE_NOT_SUPPORTED:                                u32 = 0xc000a2a3;
pub const SMB_NTSTATUS_OFFLOAD_WRITE_FILE_NOT_SUPPORTED:                               u32 = 0xc000a2a4;
pub const SMB_NTDBG_NO_STATE_CHANGE:                                                   u32 = 0xc0010001;
pub const SMB_NTDBG_APP_NOT_IDLE:                                                      u32 = 0xc0010002;
pub const SMB_NTRPC_NT_INVALID_STRING_BINDING:                                         u32 = 0xc0020001;
pub const SMB_NTRPC_NT_WRONG_KIND_OF_BINDING:                                          u32 = 0xc0020002;
pub const SMB_NTRPC_NT_INVALID_BINDING:                                                u32 = 0xc0020003;
pub const SMB_NTRPC_NT_PROTSEQ_NOT_SUPPORTED:                                          u32 = 0xc0020004;
pub const SMB_NTRPC_NT_INVALID_RPC_PROTSEQ:                                            u32 = 0xc0020005;
pub const SMB_NTRPC_NT_INVALID_STRING_UUID:                                            u32 = 0xc0020006;
pub const SMB_NTRPC_NT_INVALID_ENDPOINT_FORMAT:                                        u32 = 0xc0020007;
pub const SMB_NTRPC_NT_INVALID_NET_ADDR:                                               u32 = 0xc0020008;
pub const SMB_NTRPC_NT_NO_ENDPOINT_FOUND:                                              u32 = 0xc0020009;
pub const SMB_NTRPC_NT_INVALID_TIMEOUT:                                                u32 = 0xc002000a;
pub const SMB_NTRPC_NT_OBJECT_NOT_FOUND:                                               u32 = 0xc002000b;
pub const SMB_NTRPC_NT_ALREADY_REGISTERED:                                             u32 = 0xc002000c;
pub const SMB_NTRPC_NT_TYPE_ALREADY_REGISTERED:                                        u32 = 0xc002000d;
pub const SMB_NTRPC_NT_ALREADY_LISTENING:                                              u32 = 0xc002000e;
pub const SMB_NTRPC_NT_NO_PROTSEQS_REGISTERED:                                         u32 = 0xc002000f;
pub const SMB_NTRPC_NT_NOT_LISTENING:                                                  u32 = 0xc0020010;
pub const SMB_NTRPC_NT_UNKNOWN_MGR_TYPE:                                               u32 = 0xc0020011;
pub const SMB_NTRPC_NT_UNKNOWN_IF:                                                     u32 = 0xc0020012;
pub const SMB_NTRPC_NT_NO_BINDINGS:                                                    u32 = 0xc0020013;
pub const SMB_NTRPC_NT_NO_PROTSEQS:                                                    u32 = 0xc0020014;
pub const SMB_NTRPC_NT_CANT_CREATE_ENDPOINT:                                           u32 = 0xc0020015;
pub const SMB_NTRPC_NT_OUT_OF_RESOURCES:                                               u32 = 0xc0020016;
pub const SMB_NTRPC_NT_SERVER_UNAVAILABLE:                                             u32 = 0xc0020017;
pub const SMB_NTRPC_NT_SERVER_TOO_BUSY:                                                u32 = 0xc0020018;
pub const SMB_NTRPC_NT_INVALID_NETWORK_OPTIONS:                                        u32 = 0xc0020019;
pub const SMB_NTRPC_NT_NO_CALL_ACTIVE:                                                 u32 = 0xc002001a;
pub const SMB_NTRPC_NT_CALL_FAILED:                                                    u32 = 0xc002001b;
pub const SMB_NTRPC_NT_CALL_FAILED_DNE:                                                u32 = 0xc002001c;
pub const SMB_NTRPC_NT_PROTOCOL_ERROR:                                                 u32 = 0xc002001d;
pub const SMB_NTRPC_NT_UNSUPPORTED_TRANS_SYN:                                          u32 = 0xc002001f;
pub const SMB_NTRPC_NT_UNSUPPORTED_TYPE:                                               u32 = 0xc0020021;
pub const SMB_NTRPC_NT_INVALID_TAG:                                                    u32 = 0xc0020022;
pub const SMB_NTRPC_NT_INVALID_BOUND:                                                  u32 = 0xc0020023;
pub const SMB_NTRPC_NT_NO_ENTRY_NAME:                                                  u32 = 0xc0020024;
pub const SMB_NTRPC_NT_INVALID_NAME_SYNTAX:                                            u32 = 0xc0020025;
pub const SMB_NTRPC_NT_UNSUPPORTED_NAME_SYNTAX:                                        u32 = 0xc0020026;
pub const SMB_NTRPC_NT_UUID_NO_ADDRESS:                                                u32 = 0xc0020028;
pub const SMB_NTRPC_NT_DUPLICATE_ENDPOINT:                                             u32 = 0xc0020029;
pub const SMB_NTRPC_NT_UNKNOWN_AUTHN_TYPE:                                             u32 = 0xc002002a;
pub const SMB_NTRPC_NT_MAX_CALLS_TOO_SMALL:                                            u32 = 0xc002002b;
pub const SMB_NTRPC_NT_STRING_TOO_LONG:                                                u32 = 0xc002002c;
pub const SMB_NTRPC_NT_PROTSEQ_NOT_FOUND:                                              u32 = 0xc002002d;
pub const SMB_NTRPC_NT_PROCNUM_OUT_OF_RANGE:                                           u32 = 0xc002002e;
pub const SMB_NTRPC_NT_BINDING_HAS_NO_AUTH:                                            u32 = 0xc002002f;
pub const SMB_NTRPC_NT_UNKNOWN_AUTHN_SERVICE:                                          u32 = 0xc0020030;
pub const SMB_NTRPC_NT_UNKNOWN_AUTHN_LEVEL:                                            u32 = 0xc0020031;
pub const SMB_NTRPC_NT_INVALID_AUTH_IDENTITY:                                          u32 = 0xc0020032;
pub const SMB_NTRPC_NT_UNKNOWN_AUTHZ_SERVICE:                                          u32 = 0xc0020033;
pub const SMB_NTEPT_NT_INVALID_ENTRY:                                                  u32 = 0xc0020034;
pub const SMB_NTEPT_NT_CANT_PERFORM_OP:                                                u32 = 0xc0020035;
pub const SMB_NTEPT_NT_NOT_REGISTERED:                                                 u32 = 0xc0020036;
pub const SMB_NTRPC_NT_NOTHING_TO_EXPORT:                                              u32 = 0xc0020037;
pub const SMB_NTRPC_NT_INCOMPLETE_NAME:                                                u32 = 0xc0020038;
pub const SMB_NTRPC_NT_INVALID_VERS_OPTION:                                            u32 = 0xc0020039;
pub const SMB_NTRPC_NT_NO_MORE_MEMBERS:                                                u32 = 0xc002003a;
pub const SMB_NTRPC_NT_NOT_ALL_OBJS_UNEXPORTED:                                        u32 = 0xc002003b;
pub const SMB_NTRPC_NT_INTERFACE_NOT_FOUND:                                            u32 = 0xc002003c;
pub const SMB_NTRPC_NT_ENTRY_ALREADY_EXISTS:                                           u32 = 0xc002003d;
pub const SMB_NTRPC_NT_ENTRY_NOT_FOUND:                                                u32 = 0xc002003e;
pub const SMB_NTRPC_NT_NAME_SERVICE_UNAVAILABLE:                                       u32 = 0xc002003f;
pub const SMB_NTRPC_NT_INVALID_NAF_ID:                                                 u32 = 0xc0020040;
pub const SMB_NTRPC_NT_CANNOT_SUPPORT:                                                 u32 = 0xc0020041;
pub const SMB_NTRPC_NT_NO_CONTEXT_AVAILABLE:                                           u32 = 0xc0020042;
pub const SMB_NTRPC_NT_INTERNAL_ERROR:                                                 u32 = 0xc0020043;
pub const SMB_NTRPC_NT_ZERO_DIVIDE:                                                    u32 = 0xc0020044;
pub const SMB_NTRPC_NT_ADDRESS_ERROR:                                                  u32 = 0xc0020045;
pub const SMB_NTRPC_NT_FP_DIV_ZERO:                                                    u32 = 0xc0020046;
pub const SMB_NTRPC_NT_FP_UNDERFLOW:                                                   u32 = 0xc0020047;
pub const SMB_NTRPC_NT_FP_OVERFLOW:                                                    u32 = 0xc0020048;
pub const SMB_NTRPC_NT_CALL_IN_PROGRESS:                                               u32 = 0xc0020049;
pub const SMB_NTRPC_NT_NO_MORE_BINDINGS:                                               u32 = 0xc002004a;
pub const SMB_NTRPC_NT_GROUP_MEMBER_NOT_FOUND:                                         u32 = 0xc002004b;
pub const SMB_NTEPT_NT_CANT_CREATE:                                                    u32 = 0xc002004c;
pub const SMB_NTRPC_NT_INVALID_OBJECT:                                                 u32 = 0xc002004d;
pub const SMB_NTRPC_NT_NO_INTERFACES:                                                  u32 = 0xc002004f;
pub const SMB_NTRPC_NT_CALL_CANCELLED:                                                 u32 = 0xc0020050;
pub const SMB_NTRPC_NT_BINDING_INCOMPLETE:                                             u32 = 0xc0020051;
pub const SMB_NTRPC_NT_COMM_FAILURE:                                                   u32 = 0xc0020052;
pub const SMB_NTRPC_NT_UNSUPPORTED_AUTHN_LEVEL:                                        u32 = 0xc0020053;
pub const SMB_NTRPC_NT_NO_PRINC_NAME:                                                  u32 = 0xc0020054;
pub const SMB_NTRPC_NT_NOT_RPC_ERROR:                                                  u32 = 0xc0020055;
pub const SMB_NTRPC_NT_SEC_PKG_ERROR:                                                  u32 = 0xc0020057;
pub const SMB_NTRPC_NT_NOT_CANCELLED:                                                  u32 = 0xc0020058;
pub const SMB_NTRPC_NT_INVALID_ASYNC_HANDLE:                                           u32 = 0xc0020062;
pub const SMB_NTRPC_NT_INVALID_ASYNC_CALL:                                             u32 = 0xc0020063;
pub const SMB_NTRPC_NT_PROXY_ACCESS_DENIED:                                            u32 = 0xc0020064;
pub const SMB_NTRPC_NT_NO_MORE_ENTRIES:                                                u32 = 0xc0030001;
pub const SMB_NTRPC_NT_SS_CHAR_TRANS_OPEN_FAIL:                                        u32 = 0xc0030002;
pub const SMB_NTRPC_NT_SS_CHAR_TRANS_SHORT_FILE:                                       u32 = 0xc0030003;
pub const SMB_NTRPC_NT_SS_IN_NULL_CONTEXT:                                             u32 = 0xc0030004;
pub const SMB_NTRPC_NT_SS_CONTEXT_MISMATCH:                                            u32 = 0xc0030005;
pub const SMB_NTRPC_NT_SS_CONTEXT_DAMAGED:                                             u32 = 0xc0030006;
pub const SMB_NTRPC_NT_SS_HANDLES_MISMATCH:                                            u32 = 0xc0030007;
pub const SMB_NTRPC_NT_SS_CANNOT_GET_CALL_HANDLE:                                      u32 = 0xc0030008;
pub const SMB_NTRPC_NT_NULL_REF_POINTER:                                               u32 = 0xc0030009;
pub const SMB_NTRPC_NT_ENUM_VALUE_OUT_OF_RANGE:                                        u32 = 0xc003000a;
pub const SMB_NTRPC_NT_BYTE_COUNT_TOO_SMALL:                                           u32 = 0xc003000b;
pub const SMB_NTRPC_NT_BAD_STUB_DATA:                                                  u32 = 0xc003000c;
pub const SMB_NTRPC_NT_INVALID_ES_ACTION:                                              u32 = 0xc0030059;
pub const SMB_NTRPC_NT_WRONG_ES_VERSION:                                               u32 = 0xc003005a;
pub const SMB_NTRPC_NT_WRONG_STUB_VERSION:                                             u32 = 0xc003005b;
pub const SMB_NTRPC_NT_INVALID_PIPE_OBJECT:                                            u32 = 0xc003005c;
pub const SMB_NTRPC_NT_INVALID_PIPE_OPERATION:                                         u32 = 0xc003005d;
pub const SMB_NTRPC_NT_WRONG_PIPE_VERSION:                                             u32 = 0xc003005e;
pub const SMB_NTRPC_NT_PIPE_CLOSED:                                                    u32 = 0xc003005f;
pub const SMB_NTRPC_NT_PIPE_DISCIPLINE_ERROR:                                          u32 = 0xc0030060;
pub const SMB_NTRPC_NT_PIPE_EMPTY:                                                     u32 = 0xc0030061;
pub const SMB_NTSTATUS_PNP_BAD_MPS_TABLE:                                              u32 = 0xc0040035;
pub const SMB_NTSTATUS_PNP_TRANSLATION_FAILED:                                         u32 = 0xc0040036;
pub const SMB_NTSTATUS_PNP_IRQ_TRANSLATION_FAILED:                                     u32 = 0xc0040037;
pub const SMB_NTSTATUS_PNP_INVALID_ID:                                                 u32 = 0xc0040038;
pub const SMB_NTSTATUS_IO_REISSUE_AS_CACHED:                                           u32 = 0xc0040039;
pub const SMB_NTSTATUS_CTX_WINSTATION_NAME_INVALID:                                    u32 = 0xc00a0001;
pub const SMB_NTSTATUS_CTX_INVALID_PD:                                                 u32 = 0xc00a0002;
pub const SMB_NTSTATUS_CTX_PD_NOT_FOUND:                                               u32 = 0xc00a0003;
pub const SMB_NTSTATUS_CTX_CLOSE_PENDING:                                              u32 = 0xc00a0006;
pub const SMB_NTSTATUS_CTX_NO_OUTBUF:                                                  u32 = 0xc00a0007;
pub const SMB_NTSTATUS_CTX_MODEM_INF_NOT_FOUND:                                        u32 = 0xc00a0008;
pub const SMB_NTSTATUS_CTX_INVALID_MODEMNAME:                                          u32 = 0xc00a0009;
pub const SMB_NTSTATUS_CTX_RESPONSE_ERROR:                                             u32 = 0xc00a000a;
pub const SMB_NTSTATUS_CTX_MODEM_RESPONSE_TIMEOUT:                                     u32 = 0xc00a000b;
pub const SMB_NTSTATUS_CTX_MODEM_RESPONSE_NO_CARRIER:                                  u32 = 0xc00a000c;
pub const SMB_NTSTATUS_CTX_MODEM_RESPONSE_NO_DIALTONE:                                 u32 = 0xc00a000d;
pub const SMB_NTSTATUS_CTX_MODEM_RESPONSE_BUSY:                                        u32 = 0xc00a000e;
pub const SMB_NTSTATUS_CTX_MODEM_RESPONSE_VOICE:                                       u32 = 0xc00a000f;
pub const SMB_NTSTATUS_CTX_TD_ERROR:                                                   u32 = 0xc00a0010;
pub const SMB_NTSTATUS_CTX_LICENSE_CLIENT_INVALID:                                     u32 = 0xc00a0012;
pub const SMB_NTSTATUS_CTX_LICENSE_NOT_AVAILABLE:                                      u32 = 0xc00a0013;
pub const SMB_NTSTATUS_CTX_LICENSE_EXPIRED:                                            u32 = 0xc00a0014;
pub const SMB_NTSTATUS_CTX_WINSTATION_NOT_FOUND:                                       u32 = 0xc00a0015;
pub const SMB_NTSTATUS_CTX_WINSTATION_NAME_COLLISION:                                  u32 = 0xc00a0016;
pub const SMB_NTSTATUS_CTX_WINSTATION_BUSY:                                            u32 = 0xc00a0017;
pub const SMB_NTSTATUS_CTX_BAD_VIDEO_MODE:                                             u32 = 0xc00a0018;
pub const SMB_NTSTATUS_CTX_GRAPHICS_INVALID:                                           u32 = 0xc00a0022;
pub const SMB_NTSTATUS_CTX_NOT_CONSOLE:                                                u32 = 0xc00a0024;
pub const SMB_NTSTATUS_CTX_CLIENT_QUERY_TIMEOUT:                                       u32 = 0xc00a0026;
pub const SMB_NTSTATUS_CTX_CONSOLE_DISCONNECT:                                         u32 = 0xc00a0027;
pub const SMB_NTSTATUS_CTX_CONSOLE_CONNECT:                                            u32 = 0xc00a0028;
pub const SMB_NTSTATUS_CTX_SHADOW_DENIED:                                              u32 = 0xc00a002a;
pub const SMB_NTSTATUS_CTX_WINSTATION_ACCESS_DENIED:                                   u32 = 0xc00a002b;
pub const SMB_NTSTATUS_CTX_INVALID_WD:                                                 u32 = 0xc00a002e;
pub const SMB_NTSTATUS_CTX_WD_NOT_FOUND:                                               u32 = 0xc00a002f;
pub const SMB_NTSTATUS_CTX_SHADOW_INVALID:                                             u32 = 0xc00a0030;
pub const SMB_NTSTATUS_CTX_SHADOW_DISABLED:                                            u32 = 0xc00a0031;
pub const SMB_NTSTATUS_RDP_PROTOCOL_ERROR:                                             u32 = 0xc00a0032;
pub const SMB_NTSTATUS_CTX_CLIENT_LICENSE_NOT_SET:                                     u32 = 0xc00a0033;
pub const SMB_NTSTATUS_CTX_CLIENT_LICENSE_IN_USE:                                      u32 = 0xc00a0034;
pub const SMB_NTSTATUS_CTX_SHADOW_ENDED_BY_MODE_CHANGE:                                u32 = 0xc00a0035;
pub const SMB_NTSTATUS_CTX_SHADOW_NOT_RUNNING:                                         u32 = 0xc00a0036;
pub const SMB_NTSTATUS_CTX_LOGON_DISABLED:                                             u32 = 0xc00a0037;
pub const SMB_NTSTATUS_CTX_SECURITY_LAYER_ERROR:                                       u32 = 0xc00a0038;
pub const SMB_NTSTATUS_TS_INCOMPATIBLE_SESSIONS:                                       u32 = 0xc00a0039;
pub const SMB_NTSTATUS_MUI_FILE_NOT_FOUND:                                             u32 = 0xc00b0001;
pub const SMB_NTSTATUS_MUI_INVALID_FILE:                                               u32 = 0xc00b0002;
pub const SMB_NTSTATUS_MUI_INVALID_RC_CONFIG:                                          u32 = 0xc00b0003;
pub const SMB_NTSTATUS_MUI_INVALID_LOCALE_NAME:                                        u32 = 0xc00b0004;
pub const SMB_NTSTATUS_MUI_INVALID_ULTIMATEFALLBACK_NAME:                              u32 = 0xc00b0005;
pub const SMB_NTSTATUS_MUI_FILE_NOT_LOADED:                                            u32 = 0xc00b0006;
pub const SMB_NTSTATUS_RESOURCE_ENUM_USER_STOP:                                        u32 = 0xc00b0007;
pub const SMB_NTSTATUS_CLUSTER_INVALID_NODE:                                           u32 = 0xc0130001;
pub const SMB_NTSTATUS_CLUSTER_NODE_EXISTS:                                            u32 = 0xc0130002;
pub const SMB_NTSTATUS_CLUSTER_JOIN_IN_PROGRESS:                                       u32 = 0xc0130003;
pub const SMB_NTSTATUS_CLUSTER_NODE_NOT_FOUND:                                         u32 = 0xc0130004;
pub const SMB_NTSTATUS_CLUSTER_LOCAL_NODE_NOT_FOUND:                                   u32 = 0xc0130005;
pub const SMB_NTSTATUS_CLUSTER_NETWORK_EXISTS:                                         u32 = 0xc0130006;
pub const SMB_NTSTATUS_CLUSTER_NETWORK_NOT_FOUND:                                      u32 = 0xc0130007;
pub const SMB_NTSTATUS_CLUSTER_NETINTERFACE_EXISTS:                                    u32 = 0xc0130008;
pub const SMB_NTSTATUS_CLUSTER_NETINTERFACE_NOT_FOUND:                                 u32 = 0xc0130009;
pub const SMB_NTSTATUS_CLUSTER_INVALID_REQUEST:                                        u32 = 0xc013000a;
pub const SMB_NTSTATUS_CLUSTER_INVALID_NETWORK_PROVIDER:                               u32 = 0xc013000b;
pub const SMB_NTSTATUS_CLUSTER_NODE_DOWN:                                              u32 = 0xc013000c;
pub const SMB_NTSTATUS_CLUSTER_NODE_UNREACHABLE:                                       u32 = 0xc013000d;
pub const SMB_NTSTATUS_CLUSTER_NODE_NOT_MEMBER:                                        u32 = 0xc013000e;
pub const SMB_NTSTATUS_CLUSTER_JOIN_NOT_IN_PROGRESS:                                   u32 = 0xc013000f;
pub const SMB_NTSTATUS_CLUSTER_INVALID_NETWORK:                                        u32 = 0xc0130010;
pub const SMB_NTSTATUS_CLUSTER_NO_NET_ADAPTERS:                                        u32 = 0xc0130011;
pub const SMB_NTSTATUS_CLUSTER_NODE_UP:                                                u32 = 0xc0130012;
pub const SMB_NTSTATUS_CLUSTER_NODE_PAUSED:                                            u32 = 0xc0130013;
pub const SMB_NTSTATUS_CLUSTER_NODE_NOT_PAUSED:                                        u32 = 0xc0130014;
pub const SMB_NTSTATUS_CLUSTER_NO_SECURITY_CONTEXT:                                    u32 = 0xc0130015;
pub const SMB_NTSTATUS_CLUSTER_NETWORK_NOT_INTERNAL:                                   u32 = 0xc0130016;
pub const SMB_NTSTATUS_CLUSTER_POISONED:                                               u32 = 0xc0130017;
pub const SMB_NTSTATUS_ACPI_INVALID_OPCODE:                                            u32 = 0xc0140001;
pub const SMB_NTSTATUS_ACPI_STACK_OVERFLOW:                                            u32 = 0xc0140002;
pub const SMB_NTSTATUS_ACPI_ASSERT_FAILED:                                             u32 = 0xc0140003;
pub const SMB_NTSTATUS_ACPI_INVALID_INDEX:                                             u32 = 0xc0140004;
pub const SMB_NTSTATUS_ACPI_INVALID_ARGUMENT:                                          u32 = 0xc0140005;
pub const SMB_NTSTATUS_ACPI_FATAL:                                                     u32 = 0xc0140006;
pub const SMB_NTSTATUS_ACPI_INVALID_SUPERNAME:                                         u32 = 0xc0140007;
pub const SMB_NTSTATUS_ACPI_INVALID_ARGTYPE:                                           u32 = 0xc0140008;
pub const SMB_NTSTATUS_ACPI_INVALID_OBJTYPE:                                           u32 = 0xc0140009;
pub const SMB_NTSTATUS_ACPI_INVALID_TARGETTYPE:                                        u32 = 0xc014000a;
pub const SMB_NTSTATUS_ACPI_INCORRECT_ARGUMENT_COUNT:                                  u32 = 0xc014000b;
pub const SMB_NTSTATUS_ACPI_ADDRESS_NOT_MAPPED:                                        u32 = 0xc014000c;
pub const SMB_NTSTATUS_ACPI_INVALID_EVENTTYPE:                                         u32 = 0xc014000d;
pub const SMB_NTSTATUS_ACPI_HANDLER_COLLISION:                                         u32 = 0xc014000e;
pub const SMB_NTSTATUS_ACPI_INVALID_DATA:                                              u32 = 0xc014000f;
pub const SMB_NTSTATUS_ACPI_INVALID_REGION:                                            u32 = 0xc0140010;
pub const SMB_NTSTATUS_ACPI_INVALID_ACCESS_SIZE:                                       u32 = 0xc0140011;
pub const SMB_NTSTATUS_ACPI_ACQUIRE_GLOBAL_LOCK:                                       u32 = 0xc0140012;
pub const SMB_NTSTATUS_ACPI_ALREADY_INITIALIZED:                                       u32 = 0xc0140013;
pub const SMB_NTSTATUS_ACPI_NOT_INITIALIZED:                                           u32 = 0xc0140014;
pub const SMB_NTSTATUS_ACPI_INVALID_MUTEX_LEVEL:                                       u32 = 0xc0140015;
pub const SMB_NTSTATUS_ACPI_MUTEX_NOT_OWNED:                                           u32 = 0xc0140016;
pub const SMB_NTSTATUS_ACPI_MUTEX_NOT_OWNER:                                           u32 = 0xc0140017;
pub const SMB_NTSTATUS_ACPI_RS_ACCESS:                                                 u32 = 0xc0140018;
pub const SMB_NTSTATUS_ACPI_INVALID_TABLE:                                             u32 = 0xc0140019;
pub const SMB_NTSTATUS_ACPI_REG_HANDLER_FAILED:                                        u32 = 0xc0140020;
pub const SMB_NTSTATUS_ACPI_POWER_REQUEST_FAILED:                                      u32 = 0xc0140021;
pub const SMB_NTSTATUS_SXS_SECTION_NOT_FOUND:                                          u32 = 0xc0150001;
pub const SMB_NTSTATUS_SXS_CANT_GEN_ACTCTX:                                            u32 = 0xc0150002;
pub const SMB_NTSTATUS_SXS_INVALID_ACTCTXDATA_FORMAT:                                  u32 = 0xc0150003;
pub const SMB_NTSTATUS_SXS_ASSEMBLY_NOT_FOUND:                                         u32 = 0xc0150004;
pub const SMB_NTSTATUS_SXS_MANIFEST_FORMAT_ERROR:                                      u32 = 0xc0150005;
pub const SMB_NTSTATUS_SXS_MANIFEST_PARSE_ERROR:                                       u32 = 0xc0150006;
pub const SMB_NTSTATUS_SXS_ACTIVATION_CONTEXT_DISABLED:                                u32 = 0xc0150007;
pub const SMB_NTSTATUS_SXS_KEY_NOT_FOUND:                                              u32 = 0xc0150008;
pub const SMB_NTSTATUS_SXS_VERSION_CONFLICT:                                           u32 = 0xc0150009;
pub const SMB_NTSTATUS_SXS_WRONG_SECTION_TYPE:                                         u32 = 0xc015000a;
pub const SMB_NTSTATUS_SXS_THREAD_QUERIES_DISABLED:                                    u32 = 0xc015000b;
pub const SMB_NTSTATUS_SXS_ASSEMBLY_MISSING:                                           u32 = 0xc015000c;
pub const SMB_NTSTATUS_SXS_PROCESS_DEFAULT_ALREADY_SET:                                u32 = 0xc015000e;
pub const SMB_NTSTATUS_SXS_EARLY_DEACTIVATION:                                         u32 = 0xc015000f;
pub const SMB_NTSTATUS_SXS_INVALID_DEACTIVATION:                                       u32 = 0xc0150010;
pub const SMB_NTSTATUS_SXS_MULTIPLE_DEACTIVATION:                                      u32 = 0xc0150011;
pub const SMB_NTSTATUS_SXS_SYSTEM_DEFAULT_ACTIVATION_CONTEXT_EMPTY:                    u32 = 0xc0150012;
pub const SMB_NTSTATUS_SXS_PROCESS_TERMINATION_REQUESTED:                              u32 = 0xc0150013;
pub const SMB_NTSTATUS_SXS_CORRUPT_ACTIVATION_STACK:                                   u32 = 0xc0150014;
pub const SMB_NTSTATUS_SXS_CORRUPTION:                                                 u32 = 0xc0150015;
pub const SMB_NTSTATUS_SXS_INVALID_IDENTITY_ATTRIBUTE_VALUE:                           u32 = 0xc0150016;
pub const SMB_NTSTATUS_SXS_INVALID_IDENTITY_ATTRIBUTE_NAME:                            u32 = 0xc0150017;
pub const SMB_NTSTATUS_SXS_IDENTITY_DUPLICATE_ATTRIBUTE:                               u32 = 0xc0150018;
pub const SMB_NTSTATUS_SXS_IDENTITY_PARSE_ERROR:                                       u32 = 0xc0150019;
pub const SMB_NTSTATUS_SXS_COMPONENT_STORE_CORRUPT:                                    u32 = 0xc015001a;
pub const SMB_NTSTATUS_SXS_FILE_HASH_MISMATCH:                                         u32 = 0xc015001b;
pub const SMB_NTSTATUS_SXS_MANIFEST_IDENTITY_SAME_BUT_CONTENTS_DIFFERENT:              u32 = 0xc015001c;
pub const SMB_NTSTATUS_SXS_IDENTITIES_DIFFERENT:                                       u32 = 0xc015001d;
pub const SMB_NTSTATUS_SXS_ASSEMBLY_IS_NOT_A_DEPLOYMENT:                               u32 = 0xc015001e;
pub const SMB_NTSTATUS_SXS_FILE_NOT_PART_OF_ASSEMBLY:                                  u32 = 0xc015001f;
pub const SMB_NTSTATUS_ADVANCED_INSTALLER_FAILED:                                      u32 = 0xc0150020;
pub const SMB_NTSTATUS_XML_ENCODING_MISMATCH:                                          u32 = 0xc0150021;
pub const SMB_NTSTATUS_SXS_MANIFEST_TOO_BIG:                                           u32 = 0xc0150022;
pub const SMB_NTSTATUS_SXS_SETTING_NOT_REGISTERED:                                     u32 = 0xc0150023;
pub const SMB_NTSTATUS_SXS_TRANSACTION_CLOSURE_INCOMPLETE:                             u32 = 0xc0150024;
pub const SMB_NTSTATUS_SMI_PRIMITIVE_INSTALLER_FAILED:                                 u32 = 0xc0150025;
pub const SMB_NTSTATUS_GENERIC_COMMAND_FAILED:                                         u32 = 0xc0150026;
pub const SMB_NTSTATUS_SXS_FILE_HASH_MISSING:                                          u32 = 0xc0150027;
pub const SMB_NTSTATUS_TRANSACTIONAL_CONFLICT:                                         u32 = 0xc0190001;
pub const SMB_NTSTATUS_INVALID_TRANSACTION:                                            u32 = 0xc0190002;
pub const SMB_NTSTATUS_TRANSACTION_NOT_ACTIVE:                                         u32 = 0xc0190003;
pub const SMB_NTSTATUS_TM_INITIALIZATION_FAILED:                                       u32 = 0xc0190004;
pub const SMB_NTSTATUS_RM_NOT_ACTIVE:                                                  u32 = 0xc0190005;
pub const SMB_NTSTATUS_RM_METADATA_CORRUPT:                                            u32 = 0xc0190006;
pub const SMB_NTSTATUS_TRANSACTION_NOT_JOINED:                                         u32 = 0xc0190007;
pub const SMB_NTSTATUS_DIRECTORY_NOT_RM:                                               u32 = 0xc0190008;
pub const SMB_NTSTATUS_TRANSACTIONS_UNSUPPORTED_REMOTE:                                u32 = 0xc019000a;
pub const SMB_NTSTATUS_LOG_RESIZE_INVALID_SIZE:                                        u32 = 0xc019000b;
pub const SMB_NTSTATUS_REMOTE_FILE_VERSION_MISMATCH:                                   u32 = 0xc019000c;
pub const SMB_NTSTATUS_CRM_PROTOCOL_ALREADY_EXISTS:                                    u32 = 0xc019000f;
pub const SMB_NTSTATUS_TRANSACTION_PROPAGATION_FAILED:                                 u32 = 0xc0190010;
pub const SMB_NTSTATUS_CRM_PROTOCOL_NOT_FOUND:                                         u32 = 0xc0190011;
pub const SMB_NTSTATUS_TRANSACTION_SUPERIOR_EXISTS:                                    u32 = 0xc0190012;
pub const SMB_NTSTATUS_TRANSACTION_REQUEST_NOT_VALID:                                  u32 = 0xc0190013;
pub const SMB_NTSTATUS_TRANSACTION_NOT_REQUESTED:                                      u32 = 0xc0190014;
pub const SMB_NTSTATUS_TRANSACTION_ALREADY_ABORTED:                                    u32 = 0xc0190015;
pub const SMB_NTSTATUS_TRANSACTION_ALREADY_COMMITTED:                                  u32 = 0xc0190016;
pub const SMB_NTSTATUS_TRANSACTION_INVALID_MARSHALL_BUFFER:                            u32 = 0xc0190017;
pub const SMB_NTSTATUS_CURRENT_TRANSACTION_NOT_VALID:                                  u32 = 0xc0190018;
pub const SMB_NTSTATUS_LOG_GROWTH_FAILED:                                              u32 = 0xc0190019;
pub const SMB_NTSTATUS_OBJECT_NO_LONGER_EXISTS:                                        u32 = 0xc0190021;
pub const SMB_NTSTATUS_STREAM_MINIVERSION_NOT_FOUND:                                   u32 = 0xc0190022;
pub const SMB_NTSTATUS_STREAM_MINIVERSION_NOT_VALID:                                   u32 = 0xc0190023;
pub const SMB_NTSTATUS_MINIVERSION_INACCESSIBLE_FROM_SPECIFIED_TRANSACTION:            u32 = 0xc0190024;
pub const SMB_NTSTATUS_CANT_OPEN_MINIVERSION_WITH_MODIFY_INTENT:                       u32 = 0xc0190025;
pub const SMB_NTSTATUS_CANT_CREATE_MORE_STREAM_MINIVERSIONS:                           u32 = 0xc0190026;
pub const SMB_NTSTATUS_HANDLE_NO_LONGER_VALID:                                         u32 = 0xc0190028;
pub const SMB_NTSTATUS_LOG_CORRUPTION_DETECTED:                                        u32 = 0xc0190030;
pub const SMB_NTSTATUS_RM_DISCONNECTED:                                                u32 = 0xc0190032;
pub const SMB_NTSTATUS_ENLISTMENT_NOT_SUPERIOR:                                        u32 = 0xc0190033;
pub const SMB_NTSTATUS_FILE_IDENTITY_NOT_PERSISTENT:                                   u32 = 0xc0190036;
pub const SMB_NTSTATUS_CANT_BREAK_TRANSACTIONAL_DEPENDENCY:                            u32 = 0xc0190037;
pub const SMB_NTSTATUS_CANT_CROSS_RM_BOUNDARY:                                         u32 = 0xc0190038;
pub const SMB_NTSTATUS_TXF_DIR_NOT_EMPTY:                                              u32 = 0xc0190039;
pub const SMB_NTSTATUS_INDOUBT_TRANSACTIONS_EXIST:                                     u32 = 0xc019003a;
pub const SMB_NTSTATUS_TM_VOLATILE:                                                    u32 = 0xc019003b;
pub const SMB_NTSTATUS_ROLLBACK_TIMER_EXPIRED:                                         u32 = 0xc019003c;
pub const SMB_NTSTATUS_TXF_ATTRIBUTE_CORRUPT:                                          u32 = 0xc019003d;
pub const SMB_NTSTATUS_EFS_NOT_ALLOWED_IN_TRANSACTION:                                 u32 = 0xc019003e;
pub const SMB_NTSTATUS_TRANSACTIONAL_OPEN_NOT_ALLOWED:                                 u32 = 0xc019003f;
pub const SMB_NTSTATUS_TRANSACTED_MAPPING_UNSUPPORTED_REMOTE:                          u32 = 0xc0190040;
pub const SMB_NTSTATUS_TRANSACTION_REQUIRED_PROMOTION:                                 u32 = 0xc0190043;
pub const SMB_NTSTATUS_CANNOT_EXECUTE_FILE_IN_TRANSACTION:                             u32 = 0xc0190044;
pub const SMB_NTSTATUS_TRANSACTIONS_NOT_FROZEN:                                        u32 = 0xc0190045;
pub const SMB_NTSTATUS_TRANSACTION_FREEZE_IN_PROGRESS:                                 u32 = 0xc0190046;
pub const SMB_NTSTATUS_NOT_SNAPSHOT_VOLUME:                                            u32 = 0xc0190047;
pub const SMB_NTSTATUS_NO_SAVEPOINT_WITH_OPEN_FILES:                                   u32 = 0xc0190048;
pub const SMB_NTSTATUS_SPARSE_NOT_ALLOWED_IN_TRANSACTION:                              u32 = 0xc0190049;
pub const SMB_NTSTATUS_TM_IDENTITY_MISMATCH:                                           u32 = 0xc019004a;
pub const SMB_NTSTATUS_FLOATED_SECTION:                                                u32 = 0xc019004b;
pub const SMB_NTSTATUS_CANNOT_ACCEPT_TRANSACTED_WORK:                                  u32 = 0xc019004c;
pub const SMB_NTSTATUS_CANNOT_ABORT_TRANSACTIONS:                                      u32 = 0xc019004d;
pub const SMB_NTSTATUS_TRANSACTION_NOT_FOUND:                                          u32 = 0xc019004e;
pub const SMB_NTSTATUS_RESOURCEMANAGER_NOT_FOUND:                                      u32 = 0xc019004f;
pub const SMB_NTSTATUS_ENLISTMENT_NOT_FOUND:                                           u32 = 0xc0190050;
pub const SMB_NTSTATUS_TRANSACTIONMANAGER_NOT_FOUND:                                   u32 = 0xc0190051;
pub const SMB_NTSTATUS_TRANSACTIONMANAGER_NOT_ONLINE:                                  u32 = 0xc0190052;
pub const SMB_NTSTATUS_TRANSACTIONMANAGER_RECOVERY_NAME_COLLISION:                     u32 = 0xc0190053;
pub const SMB_NTSTATUS_TRANSACTION_NOT_ROOT:                                           u32 = 0xc0190054;
pub const SMB_NTSTATUS_TRANSACTION_OBJECT_EXPIRED:                                     u32 = 0xc0190055;
pub const SMB_NTSTATUS_COMPRESSION_NOT_ALLOWED_IN_TRANSACTION:                         u32 = 0xc0190056;
pub const SMB_NTSTATUS_TRANSACTION_RESPONSE_NOT_ENLISTED:                              u32 = 0xc0190057;
pub const SMB_NTSTATUS_TRANSACTION_RECORD_TOO_LONG:                                    u32 = 0xc0190058;
pub const SMB_NTSTATUS_NO_LINK_TRACKING_IN_TRANSACTION:                                u32 = 0xc0190059;
pub const SMB_NTSTATUS_OPERATION_NOT_SUPPORTED_IN_TRANSACTION:                         u32 = 0xc019005a;
pub const SMB_NTSTATUS_TRANSACTION_INTEGRITY_VIOLATED:                                 u32 = 0xc019005b;
pub const SMB_NTSTATUS_EXPIRED_HANDLE:                                                 u32 = 0xc0190060;
pub const SMB_NTSTATUS_TRANSACTION_NOT_ENLISTED:                                       u32 = 0xc0190061;
pub const SMB_NTSTATUS_LOG_SECTOR_INVALID:                                             u32 = 0xc01a0001;
pub const SMB_NTSTATUS_LOG_SECTOR_PARITY_INVALID:                                      u32 = 0xc01a0002;
pub const SMB_NTSTATUS_LOG_SECTOR_REMAPPED:                                            u32 = 0xc01a0003;
pub const SMB_NTSTATUS_LOG_BLOCK_INCOMPLETE:                                           u32 = 0xc01a0004;
pub const SMB_NTSTATUS_LOG_INVALID_RANGE:                                              u32 = 0xc01a0005;
pub const SMB_NTSTATUS_LOG_BLOCKS_EXHAUSTED:                                           u32 = 0xc01a0006;
pub const SMB_NTSTATUS_LOG_READ_CONTEXT_INVALID:                                       u32 = 0xc01a0007;
pub const SMB_NTSTATUS_LOG_RESTART_INVALID:                                            u32 = 0xc01a0008;
pub const SMB_NTSTATUS_LOG_BLOCK_VERSION:                                              u32 = 0xc01a0009;
pub const SMB_NTSTATUS_LOG_BLOCK_INVALID:                                              u32 = 0xc01a000a;
pub const SMB_NTSTATUS_LOG_READ_MODE_INVALID:                                          u32 = 0xc01a000b;
pub const SMB_NTSTATUS_LOG_METADATA_CORRUPT:                                           u32 = 0xc01a000d;
pub const SMB_NTSTATUS_LOG_METADATA_INVALID:                                           u32 = 0xc01a000e;
pub const SMB_NTSTATUS_LOG_METADATA_INCONSISTENT:                                      u32 = 0xc01a000f;
pub const SMB_NTSTATUS_LOG_RESERVATION_INVALID:                                        u32 = 0xc01a0010;
pub const SMB_NTSTATUS_LOG_CANT_DELETE:                                                u32 = 0xc01a0011;
pub const SMB_NTSTATUS_LOG_CONTAINER_LIMIT_EXCEEDED:                                   u32 = 0xc01a0012;
pub const SMB_NTSTATUS_LOG_START_OF_LOG:                                               u32 = 0xc01a0013;
pub const SMB_NTSTATUS_LOG_POLICY_ALREADY_INSTALLED:                                   u32 = 0xc01a0014;
pub const SMB_NTSTATUS_LOG_POLICY_NOT_INSTALLED:                                       u32 = 0xc01a0015;
pub const SMB_NTSTATUS_LOG_POLICY_INVALID:                                             u32 = 0xc01a0016;
pub const SMB_NTSTATUS_LOG_POLICY_CONFLICT:                                            u32 = 0xc01a0017;
pub const SMB_NTSTATUS_LOG_PINNED_ARCHIVE_TAIL:                                        u32 = 0xc01a0018;
pub const SMB_NTSTATUS_LOG_RECORD_NONEXISTENT:                                         u32 = 0xc01a0019;
pub const SMB_NTSTATUS_LOG_RECORDS_RESERVED_INVALID:                                   u32 = 0xc01a001a;
pub const SMB_NTSTATUS_LOG_SPACE_RESERVED_INVALID:                                     u32 = 0xc01a001b;
pub const SMB_NTSTATUS_LOG_TAIL_INVALID:                                               u32 = 0xc01a001c;
pub const SMB_NTSTATUS_LOG_FULL:                                                       u32 = 0xc01a001d;
pub const SMB_NTSTATUS_LOG_MULTIPLEXED:                                                u32 = 0xc01a001e;
pub const SMB_NTSTATUS_LOG_DEDICATED:                                                  u32 = 0xc01a001f;
pub const SMB_NTSTATUS_LOG_ARCHIVE_NOT_IN_PROGRESS:                                    u32 = 0xc01a0020;
pub const SMB_NTSTATUS_LOG_ARCHIVE_IN_PROGRESS:                                        u32 = 0xc01a0021;
pub const SMB_NTSTATUS_LOG_EPHEMERAL:                                                  u32 = 0xc01a0022;
pub const SMB_NTSTATUS_LOG_NOT_ENOUGH_CONTAINERS:                                      u32 = 0xc01a0023;
pub const SMB_NTSTATUS_LOG_CLIENT_ALREADY_REGISTERED:                                  u32 = 0xc01a0024;
pub const SMB_NTSTATUS_LOG_CLIENT_NOT_REGISTERED:                                      u32 = 0xc01a0025;
pub const SMB_NTSTATUS_LOG_FULL_HANDLER_IN_PROGRESS:                                   u32 = 0xc01a0026;
pub const SMB_NTSTATUS_LOG_CONTAINER_READ_FAILED:                                      u32 = 0xc01a0027;
pub const SMB_NTSTATUS_LOG_CONTAINER_WRITE_FAILED:                                     u32 = 0xc01a0028;
pub const SMB_NTSTATUS_LOG_CONTAINER_OPEN_FAILED:                                      u32 = 0xc01a0029;
pub const SMB_NTSTATUS_LOG_CONTAINER_STATE_INVALID:                                    u32 = 0xc01a002a;
pub const SMB_NTSTATUS_LOG_STATE_INVALID:                                              u32 = 0xc01a002b;
pub const SMB_NTSTATUS_LOG_PINNED:                                                     u32 = 0xc01a002c;
pub const SMB_NTSTATUS_LOG_METADATA_FLUSH_FAILED:                                      u32 = 0xc01a002d;
pub const SMB_NTSTATUS_LOG_INCONSISTENT_SECURITY:                                      u32 = 0xc01a002e;
pub const SMB_NTSTATUS_LOG_APPENDED_FLUSH_FAILED:                                      u32 = 0xc01a002f;
pub const SMB_NTSTATUS_LOG_PINNED_RESERVATION:                                         u32 = 0xc01a0030;
pub const SMB_NTSTATUS_VIDEO_HUNG_DISPLAY_DRIVER_THREAD:                               u32 = 0xc01b00ea;
pub const SMB_NTSTATUS_FLT_NO_HANDLER_DEFINED:                                         u32 = 0xc01c0001;
pub const SMB_NTSTATUS_FLT_CONTEXT_ALREADY_DEFINED:                                    u32 = 0xc01c0002;
pub const SMB_NTSTATUS_FLT_INVALID_ASYNCHRONOUS_REQUEST:                               u32 = 0xc01c0003;
pub const SMB_NTSTATUS_FLT_DISALLOW_FAST_IO:                                           u32 = 0xc01c0004;
pub const SMB_NTSTATUS_FLT_INVALID_NAME_REQUEST:                                       u32 = 0xc01c0005;
pub const SMB_NTSTATUS_FLT_NOT_SAFE_TO_POST_OPERATION:                                 u32 = 0xc01c0006;
pub const SMB_NTSTATUS_FLT_NOT_INITIALIZED:                                            u32 = 0xc01c0007;
pub const SMB_NTSTATUS_FLT_FILTER_NOT_READY:                                           u32 = 0xc01c0008;
pub const SMB_NTSTATUS_FLT_POST_OPERATION_CLEANUP:                                     u32 = 0xc01c0009;
pub const SMB_NTSTATUS_FLT_INTERNAL_ERROR:                                             u32 = 0xc01c000a;
pub const SMB_NTSTATUS_FLT_DELETING_OBJECT:                                            u32 = 0xc01c000b;
pub const SMB_NTSTATUS_FLT_MUST_BE_NONPAGED_POOL:                                      u32 = 0xc01c000c;
pub const SMB_NTSTATUS_FLT_DUPLICATE_ENTRY:                                            u32 = 0xc01c000d;
pub const SMB_NTSTATUS_FLT_CBDQ_DISABLED:                                              u32 = 0xc01c000e;
pub const SMB_NTSTATUS_FLT_DO_NOT_ATTACH:                                              u32 = 0xc01c000f;
pub const SMB_NTSTATUS_FLT_DO_NOT_DETACH:                                              u32 = 0xc01c0010;
pub const SMB_NTSTATUS_FLT_INSTANCE_ALTITUDE_COLLISION:                                u32 = 0xc01c0011;
pub const SMB_NTSTATUS_FLT_INSTANCE_NAME_COLLISION:                                    u32 = 0xc01c0012;
pub const SMB_NTSTATUS_FLT_FILTER_NOT_FOUND:                                           u32 = 0xc01c0013;
pub const SMB_NTSTATUS_FLT_VOLUME_NOT_FOUND:                                           u32 = 0xc01c0014;
pub const SMB_NTSTATUS_FLT_INSTANCE_NOT_FOUND:                                         u32 = 0xc01c0015;
pub const SMB_NTSTATUS_FLT_CONTEXT_ALLOCATION_NOT_FOUND:                               u32 = 0xc01c0016;
pub const SMB_NTSTATUS_FLT_INVALID_CONTEXT_REGISTRATION:                               u32 = 0xc01c0017;
pub const SMB_NTSTATUS_FLT_NAME_CACHE_MISS:                                            u32 = 0xc01c0018;
pub const SMB_NTSTATUS_FLT_NO_DEVICE_OBJECT:                                           u32 = 0xc01c0019;
pub const SMB_NTSTATUS_FLT_VOLUME_ALREADY_MOUNTED:                                     u32 = 0xc01c001a;
pub const SMB_NTSTATUS_FLT_ALREADY_ENLISTED:                                           u32 = 0xc01c001b;
pub const SMB_NTSTATUS_FLT_CONTEXT_ALREADY_LINKED:                                     u32 = 0xc01c001c;
pub const SMB_NTSTATUS_FLT_NO_WAITER_FOR_REPLY:                                        u32 = 0xc01c0020;
pub const SMB_NTSTATUS_MONITOR_NO_DESCRIPTOR:                                          u32 = 0xc01d0001;
pub const SMB_NTSTATUS_MONITOR_UNKNOWN_DESCRIPTOR_FORMAT:                              u32 = 0xc01d0002;
pub const SMB_NTSTATUS_MONITOR_INVALID_DESCRIPTOR_CHECKSUM:                            u32 = 0xc01d0003;
pub const SMB_NTSTATUS_MONITOR_INVALID_STANDARD_TIMING_BLOCK:                          u32 = 0xc01d0004;
pub const SMB_NTSTATUS_MONITOR_WMI_DATABLOCK_REGISTRATION_FAILED:                      u32 = 0xc01d0005;
pub const SMB_NTSTATUS_MONITOR_INVALID_SERIAL_NUMBER_MONDSC_BLOCK:                     u32 = 0xc01d0006;
pub const SMB_NTSTATUS_MONITOR_INVALID_USER_FRIENDLY_MONDSC_BLOCK:                     u32 = 0xc01d0007;
pub const SMB_NTSTATUS_MONITOR_NO_MORE_DESCRIPTOR_DATA:                                u32 = 0xc01d0008;
pub const SMB_NTSTATUS_MONITOR_INVALID_DETAILED_TIMING_BLOCK:                          u32 = 0xc01d0009;
pub const SMB_NTSTATUS_MONITOR_INVALID_MANUFACTURE_DATE:                               u32 = 0xc01d000a;
pub const SMB_NTSTATUS_GRAPHICS_NOT_EXCLUSIVE_MODE_OWNER:                              u32 = 0xc01e0000;
pub const SMB_NTSTATUS_GRAPHICS_INSUFFICIENT_DMA_BUFFER:                               u32 = 0xc01e0001;
pub const SMB_NTSTATUS_GRAPHICS_INVALID_DISPLAY_ADAPTER:                               u32 = 0xc01e0002;
pub const SMB_NTSTATUS_GRAPHICS_ADAPTER_WAS_RESET:                                     u32 = 0xc01e0003;
pub const SMB_NTSTATUS_GRAPHICS_INVALID_DRIVER_MODEL:                                  u32 = 0xc01e0004;
pub const SMB_NTSTATUS_GRAPHICS_PRESENT_MODE_CHANGED:                                  u32 = 0xc01e0005;
pub const SMB_NTSTATUS_GRAPHICS_PRESENT_OCCLUDED:                                      u32 = 0xc01e0006;
pub const SMB_NTSTATUS_GRAPHICS_PRESENT_DENIED:                                        u32 = 0xc01e0007;
pub const SMB_NTSTATUS_GRAPHICS_CANNOTCOLORCONVERT:                                    u32 = 0xc01e0008;
pub const SMB_NTSTATUS_GRAPHICS_PRESENT_REDIRECTION_DISABLED:                          u32 = 0xc01e000b;
pub const SMB_NTSTATUS_GRAPHICS_PRESENT_UNOCCLUDED:                                    u32 = 0xc01e000c;
pub const SMB_NTSTATUS_GRAPHICS_NO_VIDEO_MEMORY:                                       u32 = 0xc01e0100;
pub const SMB_NTSTATUS_GRAPHICS_CANT_LOCK_MEMORY:                                      u32 = 0xc01e0101;
pub const SMB_NTSTATUS_GRAPHICS_ALLOCATION_BUSY:                                       u32 = 0xc01e0102;
pub const SMB_NTSTATUS_GRAPHICS_TOO_MANY_REFERENCES:                                   u32 = 0xc01e0103;
pub const SMB_NTSTATUS_GRAPHICS_TRY_AGAIN_LATER:                                       u32 = 0xc01e0104;
pub const SMB_NTSTATUS_GRAPHICS_TRY_AGAIN_NOW:                                         u32 = 0xc01e0105;
pub const SMB_NTSTATUS_GRAPHICS_ALLOCATION_INVALID:                                    u32 = 0xc01e0106;
pub const SMB_NTSTATUS_GRAPHICS_UNSWIZZLING_APERTURE_UNAVAILABLE:                      u32 = 0xc01e0107;
pub const SMB_NTSTATUS_GRAPHICS_UNSWIZZLING_APERTURE_UNSUPPORTED:                      u32 = 0xc01e0108;
pub const SMB_NTSTATUS_GRAPHICS_CANT_EVICT_PINNED_ALLOCATION:                          u32 = 0xc01e0109;
pub const SMB_NTSTATUS_GRAPHICS_INVALID_ALLOCATION_USAGE:                              u32 = 0xc01e0110;
pub const SMB_NTSTATUS_GRAPHICS_CANT_RENDER_LOCKED_ALLOCATION:                         u32 = 0xc01e0111;
pub const SMB_NTSTATUS_GRAPHICS_ALLOCATION_CLOSED:                                     u32 = 0xc01e0112;
pub const SMB_NTSTATUS_GRAPHICS_INVALID_ALLOCATION_INSTANCE:                           u32 = 0xc01e0113;
pub const SMB_NTSTATUS_GRAPHICS_INVALID_ALLOCATION_HANDLE:                             u32 = 0xc01e0114;
pub const SMB_NTSTATUS_GRAPHICS_WRONG_ALLOCATION_DEVICE:                               u32 = 0xc01e0115;
pub const SMB_NTSTATUS_GRAPHICS_ALLOCATION_CONTENT_LOST:                               u32 = 0xc01e0116;
pub const SMB_NTSTATUS_GRAPHICS_GPU_EXCEPTION_ON_DEVICE:                               u32 = 0xc01e0200;
pub const SMB_NTSTATUS_GRAPHICS_INVALID_VIDPN_TOPOLOGY:                                u32 = 0xc01e0300;
pub const SMB_NTSTATUS_GRAPHICS_VIDPN_TOPOLOGY_NOT_SUPPORTED:                          u32 = 0xc01e0301;
pub const SMB_NTSTATUS_GRAPHICS_VIDPN_TOPOLOGY_CURRENTLY_NOT_SUPPORTED:                u32 = 0xc01e0302;
pub const SMB_NTSTATUS_GRAPHICS_INVALID_VIDPN:                                         u32 = 0xc01e0303;
pub const SMB_NTSTATUS_GRAPHICS_INVALID_VIDEO_PRESENT_SOURCE:                          u32 = 0xc01e0304;
pub const SMB_NTSTATUS_GRAPHICS_INVALID_VIDEO_PRESENT_TARGET:                          u32 = 0xc01e0305;
pub const SMB_NTSTATUS_GRAPHICS_VIDPN_MODALITY_NOT_SUPPORTED:                          u32 = 0xc01e0306;
pub const SMB_NTSTATUS_GRAPHICS_INVALID_VIDPN_SOURCEMODESET:                           u32 = 0xc01e0308;
pub const SMB_NTSTATUS_GRAPHICS_INVALID_VIDPN_TARGETMODESET:                           u32 = 0xc01e0309;
pub const SMB_NTSTATUS_GRAPHICS_INVALID_FREQUENCY:                                     u32 = 0xc01e030a;
pub const SMB_NTSTATUS_GRAPHICS_INVALID_ACTIVE_REGION:                                 u32 = 0xc01e030b;
pub const SMB_NTSTATUS_GRAPHICS_INVALID_TOTAL_REGION:                                  u32 = 0xc01e030c;
pub const SMB_NTSTATUS_GRAPHICS_INVALID_VIDEO_PRESENT_SOURCE_MODE:                     u32 = 0xc01e0310;
pub const SMB_NTSTATUS_GRAPHICS_INVALID_VIDEO_PRESENT_TARGET_MODE:                     u32 = 0xc01e0311;
pub const SMB_NTSTATUS_GRAPHICS_PINNED_MODE_MUST_REMAIN_IN_SET:                        u32 = 0xc01e0312;
pub const SMB_NTSTATUS_GRAPHICS_PATH_ALREADY_IN_TOPOLOGY:                              u32 = 0xc01e0313;
pub const SMB_NTSTATUS_GRAPHICS_MODE_ALREADY_IN_MODESET:                               u32 = 0xc01e0314;
pub const SMB_NTSTATUS_GRAPHICS_INVALID_VIDEOPRESENTSOURCESET:                         u32 = 0xc01e0315;
pub const SMB_NTSTATUS_GRAPHICS_INVALID_VIDEOPRESENTTARGETSET:                         u32 = 0xc01e0316;
pub const SMB_NTSTATUS_GRAPHICS_SOURCE_ALREADY_IN_SET:                                 u32 = 0xc01e0317;
pub const SMB_NTSTATUS_GRAPHICS_TARGET_ALREADY_IN_SET:                                 u32 = 0xc01e0318;
pub const SMB_NTSTATUS_GRAPHICS_INVALID_VIDPN_PRESENT_PATH:                            u32 = 0xc01e0319;
pub const SMB_NTSTATUS_GRAPHICS_NO_RECOMMENDED_VIDPN_TOPOLOGY:                         u32 = 0xc01e031a;
pub const SMB_NTSTATUS_GRAPHICS_INVALID_MONITOR_FREQUENCYRANGESET:                     u32 = 0xc01e031b;
pub const SMB_NTSTATUS_GRAPHICS_INVALID_MONITOR_FREQUENCYRANGE:                        u32 = 0xc01e031c;
pub const SMB_NTSTATUS_GRAPHICS_FREQUENCYRANGE_NOT_IN_SET:                             u32 = 0xc01e031d;
pub const SMB_NTSTATUS_GRAPHICS_FREQUENCYRANGE_ALREADY_IN_SET:                         u32 = 0xc01e031f;
pub const SMB_NTSTATUS_GRAPHICS_STALE_MODESET:                                         u32 = 0xc01e0320;
pub const SMB_NTSTATUS_GRAPHICS_INVALID_MONITOR_SOURCEMODESET:                         u32 = 0xc01e0321;
pub const SMB_NTSTATUS_GRAPHICS_INVALID_MONITOR_SOURCE_MODE:                           u32 = 0xc01e0322;
pub const SMB_NTSTATUS_GRAPHICS_NO_RECOMMENDED_FUNCTIONAL_VIDPN:                       u32 = 0xc01e0323;
pub const SMB_NTSTATUS_GRAPHICS_MODE_ID_MUST_BE_UNIQUE:                                u32 = 0xc01e0324;
pub const SMB_NTSTATUS_GRAPHICS_EMPTY_ADAPTER_MONITOR_MODE_SUPPORT_INTERSECTION:       u32 = 0xc01e0325;
pub const SMB_NTSTATUS_GRAPHICS_VIDEO_PRESENT_TARGETS_LESS_THAN_SOURCES:               u32 = 0xc01e0326;
pub const SMB_NTSTATUS_GRAPHICS_PATH_NOT_IN_TOPOLOGY:                                  u32 = 0xc01e0327;
pub const SMB_NTSTATUS_GRAPHICS_ADAPTER_MUST_HAVE_AT_LEAST_ONE_SOURCE:                 u32 = 0xc01e0328;
pub const SMB_NTSTATUS_GRAPHICS_ADAPTER_MUST_HAVE_AT_LEAST_ONE_TARGET:                 u32 = 0xc01e0329;
pub const SMB_NTSTATUS_GRAPHICS_INVALID_MONITORDESCRIPTORSET:                          u32 = 0xc01e032a;
pub const SMB_NTSTATUS_GRAPHICS_INVALID_MONITORDESCRIPTOR:                             u32 = 0xc01e032b;
pub const SMB_NTSTATUS_GRAPHICS_MONITORDESCRIPTOR_NOT_IN_SET:                          u32 = 0xc01e032c;
pub const SMB_NTSTATUS_GRAPHICS_MONITORDESCRIPTOR_ALREADY_IN_SET:                      u32 = 0xc01e032d;
pub const SMB_NTSTATUS_GRAPHICS_MONITORDESCRIPTOR_ID_MUST_BE_UNIQUE:                   u32 = 0xc01e032e;
pub const SMB_NTSTATUS_GRAPHICS_INVALID_VIDPN_TARGET_SUBSET_TYPE:                      u32 = 0xc01e032f;
pub const SMB_NTSTATUS_GRAPHICS_RESOURCES_NOT_RELATED:                                 u32 = 0xc01e0330;
pub const SMB_NTSTATUS_GRAPHICS_SOURCE_ID_MUST_BE_UNIQUE:                              u32 = 0xc01e0331;
pub const SMB_NTSTATUS_GRAPHICS_TARGET_ID_MUST_BE_UNIQUE:                              u32 = 0xc01e0332;
pub const SMB_NTSTATUS_GRAPHICS_NO_AVAILABLE_VIDPN_TARGET:                             u32 = 0xc01e0333;
pub const SMB_NTSTATUS_GRAPHICS_MONITOR_COULD_NOT_BE_ASSOCIATED_WITH_ADAPTER:          u32 = 0xc01e0334;
pub const SMB_NTSTATUS_GRAPHICS_NO_VIDPNMGR:                                           u32 = 0xc01e0335;
pub const SMB_NTSTATUS_GRAPHICS_NO_ACTIVE_VIDPN:                                       u32 = 0xc01e0336;
pub const SMB_NTSTATUS_GRAPHICS_STALE_VIDPN_TOPOLOGY:                                  u32 = 0xc01e0337;
pub const SMB_NTSTATUS_GRAPHICS_MONITOR_NOT_CONNECTED:                                 u32 = 0xc01e0338;
pub const SMB_NTSTATUS_GRAPHICS_SOURCE_NOT_IN_TOPOLOGY:                                u32 = 0xc01e0339;
pub const SMB_NTSTATUS_GRAPHICS_INVALID_PRIMARYSURFACE_SIZE:                           u32 = 0xc01e033a;
pub const SMB_NTSTATUS_GRAPHICS_INVALID_VISIBLEREGION_SIZE:                            u32 = 0xc01e033b;
pub const SMB_NTSTATUS_GRAPHICS_INVALID_STRIDE:                                        u32 = 0xc01e033c;
pub const SMB_NTSTATUS_GRAPHICS_INVALID_PIXELFORMAT:                                   u32 = 0xc01e033d;
pub const SMB_NTSTATUS_GRAPHICS_INVALID_COLORBASIS:                                    u32 = 0xc01e033e;
pub const SMB_NTSTATUS_GRAPHICS_INVALID_PIXELVALUEACCESSMODE:                          u32 = 0xc01e033f;
pub const SMB_NTSTATUS_GRAPHICS_TARGET_NOT_IN_TOPOLOGY:                                u32 = 0xc01e0340;
pub const SMB_NTSTATUS_GRAPHICS_NO_DISPLAY_MODE_MANAGEMENT_SUPPORT:                    u32 = 0xc01e0341;
pub const SMB_NTSTATUS_GRAPHICS_VIDPN_SOURCE_IN_USE:                                   u32 = 0xc01e0342;
pub const SMB_NTSTATUS_GRAPHICS_CANT_ACCESS_ACTIVE_VIDPN:                              u32 = 0xc01e0343;
pub const SMB_NTSTATUS_GRAPHICS_INVALID_PATH_IMPORTANCE_ORDINAL:                       u32 = 0xc01e0344;
pub const SMB_NTSTATUS_GRAPHICS_INVALID_PATH_CONTENT_GEOMETRY_TRANSFORMATION:          u32 = 0xc01e0345;
pub const SMB_NTSTATUS_GRAPHICS_PATH_CONTENT_GEOMETRY_TRANSFORMATION_NOT_SUPPORTED:    u32 = 0xc01e0346;
pub const SMB_NTSTATUS_GRAPHICS_INVALID_GAMMA_RAMP:                                    u32 = 0xc01e0347;
pub const SMB_NTSTATUS_GRAPHICS_GAMMA_RAMP_NOT_SUPPORTED:                              u32 = 0xc01e0348;
pub const SMB_NTSTATUS_GRAPHICS_MULTISAMPLING_NOT_SUPPORTED:                           u32 = 0xc01e0349;
pub const SMB_NTSTATUS_GRAPHICS_MODE_NOT_IN_MODESET:                                   u32 = 0xc01e034a;
pub const SMB_NTSTATUS_GRAPHICS_INVALID_VIDPN_TOPOLOGY_RECOMMENDATION_REASON:          u32 = 0xc01e034d;
pub const SMB_NTSTATUS_GRAPHICS_INVALID_PATH_CONTENT_TYPE:                             u32 = 0xc01e034e;
pub const SMB_NTSTATUS_GRAPHICS_INVALID_COPYPROTECTION_TYPE:                           u32 = 0xc01e034f;
pub const SMB_NTSTATUS_GRAPHICS_UNASSIGNED_MODESET_ALREADY_EXISTS:                     u32 = 0xc01e0350;
pub const SMB_NTSTATUS_GRAPHICS_INVALID_SCANLINE_ORDERING:                             u32 = 0xc01e0352;
pub const SMB_NTSTATUS_GRAPHICS_TOPOLOGY_CHANGES_NOT_ALLOWED:                          u32 = 0xc01e0353;
pub const SMB_NTSTATUS_GRAPHICS_NO_AVAILABLE_IMPORTANCE_ORDINALS:                      u32 = 0xc01e0354;
pub const SMB_NTSTATUS_GRAPHICS_INCOMPATIBLE_PRIVATE_FORMAT:                           u32 = 0xc01e0355;
pub const SMB_NTSTATUS_GRAPHICS_INVALID_MODE_PRUNING_ALGORITHM:                        u32 = 0xc01e0356;
pub const SMB_NTSTATUS_GRAPHICS_INVALID_MONITOR_CAPABILITY_ORIGIN:                     u32 = 0xc01e0357;
pub const SMB_NTSTATUS_GRAPHICS_INVALID_MONITOR_FREQUENCYRANGE_CONSTRAINT:             u32 = 0xc01e0358;
pub const SMB_NTSTATUS_GRAPHICS_MAX_NUM_PATHS_REACHED:                                 u32 = 0xc01e0359;
pub const SMB_NTSTATUS_GRAPHICS_CANCEL_VIDPN_TOPOLOGY_AUGMENTATION:                    u32 = 0xc01e035a;
pub const SMB_NTSTATUS_GRAPHICS_INVALID_CLIENT_TYPE:                                   u32 = 0xc01e035b;
pub const SMB_NTSTATUS_GRAPHICS_CLIENTVIDPN_NOT_SET:                                   u32 = 0xc01e035c;
pub const SMB_NTSTATUS_GRAPHICS_SPECIFIED_CHILD_ALREADY_CONNECTED:                     u32 = 0xc01e0400;
pub const SMB_NTSTATUS_GRAPHICS_CHILD_DESCRIPTOR_NOT_SUPPORTED:                        u32 = 0xc01e0401;
pub const SMB_NTSTATUS_GRAPHICS_NOT_A_LINKED_ADAPTER:                                  u32 = 0xc01e0430;
pub const SMB_NTSTATUS_GRAPHICS_LEADLINK_NOT_ENUMERATED:                               u32 = 0xc01e0431;
pub const SMB_NTSTATUS_GRAPHICS_CHAINLINKS_NOT_ENUMERATED:                             u32 = 0xc01e0432;
pub const SMB_NTSTATUS_GRAPHICS_ADAPTER_CHAIN_NOT_READY:                               u32 = 0xc01e0433;
pub const SMB_NTSTATUS_GRAPHICS_CHAINLINKS_NOT_STARTED:                                u32 = 0xc01e0434;
pub const SMB_NTSTATUS_GRAPHICS_CHAINLINKS_NOT_POWERED_ON:                             u32 = 0xc01e0435;
pub const SMB_NTSTATUS_GRAPHICS_INCONSISTENT_DEVICE_LINK_STATE:                        u32 = 0xc01e0436;
pub const SMB_NTSTATUS_GRAPHICS_NOT_POST_DEVICE_DRIVER:                                u32 = 0xc01e0438;
pub const SMB_NTSTATUS_GRAPHICS_ADAPTER_ACCESS_NOT_EXCLUDED:                           u32 = 0xc01e043b;
pub const SMB_NTSTATUS_GRAPHICS_OPM_NOT_SUPPORTED:                                     u32 = 0xc01e0500;
pub const SMB_NTSTATUS_GRAPHICS_COPP_NOT_SUPPORTED:                                    u32 = 0xc01e0501;
pub const SMB_NTSTATUS_GRAPHICS_UAB_NOT_SUPPORTED:                                     u32 = 0xc01e0502;
pub const SMB_NTSTATUS_GRAPHICS_OPM_INVALID_ENCRYPTED_PARAMETERS:                      u32 = 0xc01e0503;
pub const SMB_NTSTATUS_GRAPHICS_OPM_PARAMETER_ARRAY_TOO_SMALL:                         u32 = 0xc01e0504;
pub const SMB_NTSTATUS_GRAPHICS_OPM_NO_PROTECTED_OUTPUTS_EXIST:                        u32 = 0xc01e0505;
pub const SMB_NTSTATUS_GRAPHICS_PVP_NO_DISPLAY_DEVICE_CORRESPONDS_TO_NAME:             u32 = 0xc01e0506;
pub const SMB_NTSTATUS_GRAPHICS_PVP_DISPLAY_DEVICE_NOT_ATTACHED_TO_DESKTOP:            u32 = 0xc01e0507;
pub const SMB_NTSTATUS_GRAPHICS_PVP_MIRRORING_DEVICES_NOT_SUPPORTED:                   u32 = 0xc01e0508;
pub const SMB_NTSTATUS_GRAPHICS_OPM_INVALID_POINTER:                                   u32 = 0xc01e050a;
pub const SMB_NTSTATUS_GRAPHICS_OPM_INTERNAL_ERROR:                                    u32 = 0xc01e050b;
pub const SMB_NTSTATUS_GRAPHICS_OPM_INVALID_HANDLE:                                    u32 = 0xc01e050c;
pub const SMB_NTSTATUS_GRAPHICS_PVP_NO_MONITORS_CORRESPOND_TO_DISPLAY_DEVICE:          u32 = 0xc01e050d;
pub const SMB_NTSTATUS_GRAPHICS_PVP_INVALID_CERTIFICATE_LENGTH:                        u32 = 0xc01e050e;
pub const SMB_NTSTATUS_GRAPHICS_OPM_SPANNING_MODE_ENABLED:                             u32 = 0xc01e050f;
pub const SMB_NTSTATUS_GRAPHICS_OPM_THEATER_MODE_ENABLED:                              u32 = 0xc01e0510;
pub const SMB_NTSTATUS_GRAPHICS_PVP_HFS_FAILED:                                        u32 = 0xc01e0511;
pub const SMB_NTSTATUS_GRAPHICS_OPM_INVALID_SRM:                                       u32 = 0xc01e0512;
pub const SMB_NTSTATUS_GRAPHICS_OPM_OUTPUT_DOES_NOT_SUPPORT_HDCP:                      u32 = 0xc01e0513;
pub const SMB_NTSTATUS_GRAPHICS_OPM_OUTPUT_DOES_NOT_SUPPORT_ACP:                       u32 = 0xc01e0514;
pub const SMB_NTSTATUS_GRAPHICS_OPM_OUTPUT_DOES_NOT_SUPPORT_CGMSA:                     u32 = 0xc01e0515;
pub const SMB_NTSTATUS_GRAPHICS_OPM_HDCP_SRM_NEVER_SET:                                u32 = 0xc01e0516;
pub const SMB_NTSTATUS_GRAPHICS_OPM_RESOLUTION_TOO_HIGH:                               u32 = 0xc01e0517;
pub const SMB_NTSTATUS_GRAPHICS_OPM_ALL_HDCP_HARDWARE_ALREADY_IN_USE:                  u32 = 0xc01e0518;
pub const SMB_NTSTATUS_GRAPHICS_OPM_PROTECTED_OUTPUT_NO_LONGER_EXISTS:                 u32 = 0xc01e051a;
pub const SMB_NTSTATUS_GRAPHICS_OPM_SESSION_TYPE_CHANGE_IN_PROGRESS:                   u32 = 0xc01e051b;
pub const SMB_NTSTATUS_GRAPHICS_OPM_PROTECTED_OUTPUT_DOES_NOT_HAVE_COPP_SEMANTICS:     u32 = 0xc01e051c;
pub const SMB_NTSTATUS_GRAPHICS_OPM_INVALID_INFORMATION_REQUEST:                       u32 = 0xc01e051d;
pub const SMB_NTSTATUS_GRAPHICS_OPM_DRIVER_INTERNAL_ERROR:                             u32 = 0xc01e051e;
pub const SMB_NTSTATUS_GRAPHICS_OPM_PROTECTED_OUTPUT_DOES_NOT_HAVE_OPM_SEMANTICS:      u32 = 0xc01e051f;
pub const SMB_NTSTATUS_GRAPHICS_OPM_SIGNALING_NOT_SUPPORTED:                           u32 = 0xc01e0520;
pub const SMB_NTSTATUS_GRAPHICS_OPM_INVALID_CONFIGURATION_REQUEST:                     u32 = 0xc01e0521;
pub const SMB_NTSTATUS_GRAPHICS_I2C_NOT_SUPPORTED:                                     u32 = 0xc01e0580;
pub const SMB_NTSTATUS_GRAPHICS_I2C_DEVICE_DOES_NOT_EXIST:                             u32 = 0xc01e0581;
pub const SMB_NTSTATUS_GRAPHICS_I2C_ERROR_TRANSMITTING_DATA:                           u32 = 0xc01e0582;
pub const SMB_NTSTATUS_GRAPHICS_I2C_ERROR_RECEIVING_DATA:                              u32 = 0xc01e0583;
pub const SMB_NTSTATUS_GRAPHICS_DDCCI_VCP_NOT_SUPPORTED:                               u32 = 0xc01e0584;
pub const SMB_NTSTATUS_GRAPHICS_DDCCI_INVALID_DATA:                                    u32 = 0xc01e0585;
pub const SMB_NTSTATUS_GRAPHICS_DDCCI_MONITOR_RETURNED_INVALID_TIMING_STATUS_BYTE:     u32 = 0xc01e0586;
pub const SMB_NTSTATUS_GRAPHICS_DDCCI_INVALID_CAPABILITIES_STRING:                     u32 = 0xc01e0587;
pub const SMB_NTSTATUS_GRAPHICS_MCA_INTERNAL_ERROR:                                    u32 = 0xc01e0588;
pub const SMB_NTSTATUS_GRAPHICS_DDCCI_INVALID_MESSAGE_COMMAND:                         u32 = 0xc01e0589;
pub const SMB_NTSTATUS_GRAPHICS_DDCCI_INVALID_MESSAGE_LENGTH:                          u32 = 0xc01e058a;
pub const SMB_NTSTATUS_GRAPHICS_DDCCI_INVALID_MESSAGE_CHECKSUM:                        u32 = 0xc01e058b;
pub const SMB_NTSTATUS_GRAPHICS_INVALID_PHYSICAL_MONITOR_HANDLE:                       u32 = 0xc01e058c;
pub const SMB_NTSTATUS_GRAPHICS_MONITOR_NO_LONGER_EXISTS:                              u32 = 0xc01e058d;
pub const SMB_NTSTATUS_GRAPHICS_ONLY_CONSOLE_SESSION_SUPPORTED:                        u32 = 0xc01e05e0;
pub const SMB_NTSTATUS_GRAPHICS_NO_DISPLAY_DEVICE_CORRESPONDS_TO_NAME:                 u32 = 0xc01e05e1;
pub const SMB_NTSTATUS_GRAPHICS_DISPLAY_DEVICE_NOT_ATTACHED_TO_DESKTOP:                u32 = 0xc01e05e2;
pub const SMB_NTSTATUS_GRAPHICS_MIRRORING_DEVICES_NOT_SUPPORTED:                       u32 = 0xc01e05e3;
pub const SMB_NTSTATUS_GRAPHICS_INVALID_POINTER:                                       u32 = 0xc01e05e4;
pub const SMB_NTSTATUS_GRAPHICS_NO_MONITORS_CORRESPOND_TO_DISPLAY_DEVICE:              u32 = 0xc01e05e5;
pub const SMB_NTSTATUS_GRAPHICS_PARAMETER_ARRAY_TOO_SMALL:                             u32 = 0xc01e05e6;
pub const SMB_NTSTATUS_GRAPHICS_INTERNAL_ERROR:                                        u32 = 0xc01e05e7;
pub const SMB_NTSTATUS_GRAPHICS_SESSION_TYPE_CHANGE_IN_PROGRESS:                       u32 = 0xc01e05e8;
pub const SMB_NTSTATUS_FVE_LOCKED_VOLUME:                                              u32 = 0xc0210000;
pub const SMB_NTSTATUS_FVE_NOT_ENCRYPTED:                                              u32 = 0xc0210001;
pub const SMB_NTSTATUS_FVE_BAD_INFORMATION:                                            u32 = 0xc0210002;
pub const SMB_NTSTATUS_FVE_TOO_SMALL:                                                  u32 = 0xc0210003;
pub const SMB_NTSTATUS_FVE_FAILED_WRONG_FS:                                            u32 = 0xc0210004;
pub const SMB_NTSTATUS_FVE_FAILED_BAD_FS:                                              u32 = 0xc0210005;
pub const SMB_NTSTATUS_FVE_FS_NOT_EXTENDED:                                            u32 = 0xc0210006;
pub const SMB_NTSTATUS_FVE_FS_MOUNTED:                                                 u32 = 0xc0210007;
pub const SMB_NTSTATUS_FVE_NO_LICENSE:                                                 u32 = 0xc0210008;
pub const SMB_NTSTATUS_FVE_ACTION_NOT_ALLOWED:                                         u32 = 0xc0210009;
pub const SMB_NTSTATUS_FVE_BAD_DATA:                                                   u32 = 0xc021000a;
pub const SMB_NTSTATUS_FVE_VOLUME_NOT_BOUND:                                           u32 = 0xc021000b;
pub const SMB_NTSTATUS_FVE_NOT_DATA_VOLUME:                                            u32 = 0xc021000c;
pub const SMB_NTSTATUS_FVE_CONV_READ_ERROR:                                            u32 = 0xc021000d;
pub const SMB_NTSTATUS_FVE_CONV_WRITE_ERROR:                                           u32 = 0xc021000e;
pub const SMB_NTSTATUS_FVE_OVERLAPPED_UPDATE:                                          u32 = 0xc021000f;
pub const SMB_NTSTATUS_FVE_FAILED_SECTOR_SIZE:                                         u32 = 0xc0210010;
pub const SMB_NTSTATUS_FVE_FAILED_AUTHENTICATION:                                      u32 = 0xc0210011;
pub const SMB_NTSTATUS_FVE_NOT_OS_VOLUME:                                              u32 = 0xc0210012;
pub const SMB_NTSTATUS_FVE_KEYFILE_NOT_FOUND:                                          u32 = 0xc0210013;
pub const SMB_NTSTATUS_FVE_KEYFILE_INVALID:                                            u32 = 0xc0210014;
pub const SMB_NTSTATUS_FVE_KEYFILE_NO_VMK:                                             u32 = 0xc0210015;
pub const SMB_NTSTATUS_FVE_TPM_DISABLED:                                               u32 = 0xc0210016;
pub const SMB_NTSTATUS_FVE_TPM_SRK_AUTH_NOT_ZERO:                                      u32 = 0xc0210017;
pub const SMB_NTSTATUS_FVE_TPM_INVALID_PCR:                                            u32 = 0xc0210018;
pub const SMB_NTSTATUS_FVE_TPM_NO_VMK:                                                 u32 = 0xc0210019;
pub const SMB_NTSTATUS_FVE_PIN_INVALID:                                                u32 = 0xc021001a;
pub const SMB_NTSTATUS_FVE_AUTH_INVALID_APPLICATION:                                   u32 = 0xc021001b;
pub const SMB_NTSTATUS_FVE_AUTH_INVALID_CONFIG:                                        u32 = 0xc021001c;
pub const SMB_NTSTATUS_FVE_DEBUGGER_ENABLED:                                           u32 = 0xc021001d;
pub const SMB_NTSTATUS_FVE_DRY_RUN_FAILED:                                             u32 = 0xc021001e;
pub const SMB_NTSTATUS_FVE_BAD_METADATA_POINTER:                                       u32 = 0xc021001f;
pub const SMB_NTSTATUS_FVE_OLD_METADATA_COPY:                                          u32 = 0xc0210020;
pub const SMB_NTSTATUS_FVE_REBOOT_REQUIRED:                                            u32 = 0xc0210021;
pub const SMB_NTSTATUS_FVE_RAW_ACCESS:                                                 u32 = 0xc0210022;
pub const SMB_NTSTATUS_FVE_RAW_BLOCKED:                                                u32 = 0xc0210023;
pub const SMB_NTSTATUS_FVE_NO_FEATURE_LICENSE:                                         u32 = 0xc0210026;
pub const SMB_NTSTATUS_FVE_POLICY_USER_DISABLE_RDV_NOT_ALLOWED:                        u32 = 0xc0210027;
pub const SMB_NTSTATUS_FVE_CONV_RECOVERY_FAILED:                                       u32 = 0xc0210028;
pub const SMB_NTSTATUS_FVE_VIRTUALIZED_SPACE_TOO_BIG:                                  u32 = 0xc0210029;
pub const SMB_NTSTATUS_FVE_VOLUME_TOO_SMALL:                                           u32 = 0xc0210030;
pub const SMB_NTSTATUS_FWP_CALLOUT_NOT_FOUND:                                          u32 = 0xc0220001;
pub const SMB_NTSTATUS_FWP_CONDITION_NOT_FOUND:                                        u32 = 0xc0220002;
pub const SMB_NTSTATUS_FWP_FILTER_NOT_FOUND:                                           u32 = 0xc0220003;
pub const SMB_NTSTATUS_FWP_LAYER_NOT_FOUND:                                            u32 = 0xc0220004;
pub const SMB_NTSTATUS_FWP_PROVIDER_NOT_FOUND:                                         u32 = 0xc0220005;
pub const SMB_NTSTATUS_FWP_PROVIDER_CONTEXT_NOT_FOUND:                                 u32 = 0xc0220006;
pub const SMB_NTSTATUS_FWP_SUBLAYER_NOT_FOUND:                                         u32 = 0xc0220007;
pub const SMB_NTSTATUS_FWP_NOT_FOUND:                                                  u32 = 0xc0220008;
pub const SMB_NTSTATUS_FWP_ALREADY_EXISTS:                                             u32 = 0xc0220009;
pub const SMB_NTSTATUS_FWP_IN_USE:                                                     u32 = 0xc022000a;
pub const SMB_NTSTATUS_FWP_DYNAMIC_SESSION_IN_PROGRESS:                                u32 = 0xc022000b;
pub const SMB_NTSTATUS_FWP_WRONG_SESSION:                                              u32 = 0xc022000c;
pub const SMB_NTSTATUS_FWP_NO_TXN_IN_PROGRESS:                                         u32 = 0xc022000d;
pub const SMB_NTSTATUS_FWP_TXN_IN_PROGRESS:                                            u32 = 0xc022000e;
pub const SMB_NTSTATUS_FWP_TXN_ABORTED:                                                u32 = 0xc022000f;
pub const SMB_NTSTATUS_FWP_SESSION_ABORTED:                                            u32 = 0xc0220010;
pub const SMB_NTSTATUS_FWP_INCOMPATIBLE_TXN:                                           u32 = 0xc0220011;
pub const SMB_NTSTATUS_FWP_TIMEOUT:                                                    u32 = 0xc0220012;
pub const SMB_NTSTATUS_FWP_NET_EVENTS_DISABLED:                                        u32 = 0xc0220013;
pub const SMB_NTSTATUS_FWP_INCOMPATIBLE_LAYER:                                         u32 = 0xc0220014;
pub const SMB_NTSTATUS_FWP_KM_CLIENTS_ONLY:                                            u32 = 0xc0220015;
pub const SMB_NTSTATUS_FWP_LIFETIME_MISMATCH:                                          u32 = 0xc0220016;
pub const SMB_NTSTATUS_FWP_BUILTIN_OBJECT:                                             u32 = 0xc0220017;
pub const SMB_NTSTATUS_FWP_TOO_MANY_BOOTTIME_FILTERS:                                  u32 = 0xc0220018;
pub const SMB_NTSTATUS_FWP_NOTIFICATION_DROPPED:                                       u32 = 0xc0220019;
pub const SMB_NTSTATUS_FWP_TRAFFIC_MISMATCH:                                           u32 = 0xc022001a;
pub const SMB_NTSTATUS_FWP_INCOMPATIBLE_SA_STATE:                                      u32 = 0xc022001b;
pub const SMB_NTSTATUS_FWP_NULL_POINTER:                                               u32 = 0xc022001c;
pub const SMB_NTSTATUS_FWP_INVALID_ENUMERATOR:                                         u32 = 0xc022001d;
pub const SMB_NTSTATUS_FWP_INVALID_FLAGS:                                              u32 = 0xc022001e;
pub const SMB_NTSTATUS_FWP_INVALID_NET_MASK:                                           u32 = 0xc022001f;
pub const SMB_NTSTATUS_FWP_INVALID_RANGE:                                              u32 = 0xc0220020;
pub const SMB_NTSTATUS_FWP_INVALID_INTERVAL:                                           u32 = 0xc0220021;
pub const SMB_NTSTATUS_FWP_ZERO_LENGTH_ARRAY:                                          u32 = 0xc0220022;
pub const SMB_NTSTATUS_FWP_NULL_DISPLAY_NAME:                                          u32 = 0xc0220023;
pub const SMB_NTSTATUS_FWP_INVALID_ACTION_TYPE:                                        u32 = 0xc0220024;
pub const SMB_NTSTATUS_FWP_INVALID_WEIGHT:                                             u32 = 0xc0220025;
pub const SMB_NTSTATUS_FWP_MATCH_TYPE_MISMATCH:                                        u32 = 0xc0220026;
pub const SMB_NTSTATUS_FWP_TYPE_MISMATCH:                                              u32 = 0xc0220027;
pub const SMB_NTSTATUS_FWP_OUT_OF_BOUNDS:                                              u32 = 0xc0220028;
pub const SMB_NTSTATUS_FWP_RESERVED:                                                   u32 = 0xc0220029;
pub const SMB_NTSTATUS_FWP_DUPLICATE_CONDITION:                                        u32 = 0xc022002a;
pub const SMB_NTSTATUS_FWP_DUPLICATE_KEYMOD:                                           u32 = 0xc022002b;
pub const SMB_NTSTATUS_FWP_ACTION_INCOMPATIBLE_WITH_LAYER:                             u32 = 0xc022002c;
pub const SMB_NTSTATUS_FWP_ACTION_INCOMPATIBLE_WITH_SUBLAYER:                          u32 = 0xc022002d;
pub const SMB_NTSTATUS_FWP_CONTEXT_INCOMPATIBLE_WITH_LAYER:                            u32 = 0xc022002e;
pub const SMB_NTSTATUS_FWP_CONTEXT_INCOMPATIBLE_WITH_CALLOUT:                          u32 = 0xc022002f;
pub const SMB_NTSTATUS_FWP_INCOMPATIBLE_AUTH_METHOD:                                   u32 = 0xc0220030;
pub const SMB_NTSTATUS_FWP_INCOMPATIBLE_DH_GROUP:                                      u32 = 0xc0220031;
pub const SMB_NTSTATUS_FWP_EM_NOT_SUPPORTED:                                           u32 = 0xc0220032;
pub const SMB_NTSTATUS_FWP_NEVER_MATCH:                                                u32 = 0xc0220033;
pub const SMB_NTSTATUS_FWP_PROVIDER_CONTEXT_MISMATCH:                                  u32 = 0xc0220034;
pub const SMB_NTSTATUS_FWP_INVALID_PARAMETER:                                          u32 = 0xc0220035;
pub const SMB_NTSTATUS_FWP_TOO_MANY_SUBLAYERS:                                         u32 = 0xc0220036;
pub const SMB_NTSTATUS_FWP_CALLOUT_NOTIFICATION_FAILED:                                u32 = 0xc0220037;
pub const SMB_NTSTATUS_FWP_INCOMPATIBLE_AUTH_CONFIG:                                   u32 = 0xc0220038;
pub const SMB_NTSTATUS_FWP_INCOMPATIBLE_CIPHER_CONFIG:                                 u32 = 0xc0220039;
pub const SMB_NTSTATUS_FWP_DUPLICATE_AUTH_METHOD:                                      u32 = 0xc022003c;
pub const SMB_NTSTATUS_FWP_TCPIP_NOT_READY:                                            u32 = 0xc0220100;
pub const SMB_NTSTATUS_FWP_INJECT_HANDLE_CLOSING:                                      u32 = 0xc0220101;
pub const SMB_NTSTATUS_FWP_INJECT_HANDLE_STALE:                                        u32 = 0xc0220102;
pub const SMB_NTSTATUS_FWP_CANNOT_PEND:                                                u32 = 0xc0220103;
pub const SMB_NTSTATUS_NDIS_CLOSING:                                                   u32 = 0xc0230002;
pub const SMB_NTSTATUS_NDIS_BAD_VERSION:                                               u32 = 0xc0230004;
pub const SMB_NTSTATUS_NDIS_BAD_CHARACTERISTICS:                                       u32 = 0xc0230005;
pub const SMB_NTSTATUS_NDIS_ADAPTER_NOT_FOUND:                                         u32 = 0xc0230006;
pub const SMB_NTSTATUS_NDIS_OPEN_FAILED:                                               u32 = 0xc0230007;
pub const SMB_NTSTATUS_NDIS_DEVICE_FAILED:                                             u32 = 0xc0230008;
pub const SMB_NTSTATUS_NDIS_MULTICAST_FULL:                                            u32 = 0xc0230009;
pub const SMB_NTSTATUS_NDIS_MULTICAST_EXISTS:                                          u32 = 0xc023000a;
pub const SMB_NTSTATUS_NDIS_MULTICAST_NOT_FOUND:                                       u32 = 0xc023000b;
pub const SMB_NTSTATUS_NDIS_REQUEST_ABORTED:                                           u32 = 0xc023000c;
pub const SMB_NTSTATUS_NDIS_RESET_IN_PROGRESS:                                         u32 = 0xc023000d;
pub const SMB_NTSTATUS_NDIS_INVALID_PACKET:                                            u32 = 0xc023000f;
pub const SMB_NTSTATUS_NDIS_INVALID_DEVICE_REQUEST:                                    u32 = 0xc0230010;
pub const SMB_NTSTATUS_NDIS_ADAPTER_NOT_READY:                                         u32 = 0xc0230011;
pub const SMB_NTSTATUS_NDIS_INVALID_LENGTH:                                            u32 = 0xc0230014;
pub const SMB_NTSTATUS_NDIS_INVALID_DATA:                                              u32 = 0xc0230015;
pub const SMB_NTSTATUS_NDIS_BUFFER_TOO_SHORT:                                          u32 = 0xc0230016;
pub const SMB_NTSTATUS_NDIS_INVALID_OID:                                               u32 = 0xc0230017;
pub const SMB_NTSTATUS_NDIS_ADAPTER_REMOVED:                                           u32 = 0xc0230018;
pub const SMB_NTSTATUS_NDIS_UNSUPPORTED_MEDIA:                                         u32 = 0xc0230019;
pub const SMB_NTSTATUS_NDIS_GROUP_ADDRESS_IN_USE:                                      u32 = 0xc023001a;
pub const SMB_NTSTATUS_NDIS_FILE_NOT_FOUND:                                            u32 = 0xc023001b;
pub const SMB_NTSTATUS_NDIS_ERROR_READING_FILE:                                        u32 = 0xc023001c;
pub const SMB_NTSTATUS_NDIS_ALREADY_MAPPED:                                            u32 = 0xc023001d;
pub const SMB_NTSTATUS_NDIS_RESOURCE_CONFLICT:                                         u32 = 0xc023001e;
pub const SMB_NTSTATUS_NDIS_MEDIA_DISCONNECTED:                                        u32 = 0xc023001f;
pub const SMB_NTSTATUS_NDIS_INVALID_ADDRESS:                                           u32 = 0xc0230022;
pub const SMB_NTSTATUS_NDIS_PAUSED:                                                    u32 = 0xc023002a;
pub const SMB_NTSTATUS_NDIS_INTERFACE_NOT_FOUND:                                       u32 = 0xc023002b;
pub const SMB_NTSTATUS_NDIS_UNSUPPORTED_REVISION:                                      u32 = 0xc023002c;
pub const SMB_NTSTATUS_NDIS_INVALID_PORT:                                              u32 = 0xc023002d;
pub const SMB_NTSTATUS_NDIS_INVALID_PORT_STATE:                                        u32 = 0xc023002e;
pub const SMB_NTSTATUS_NDIS_LOW_POWER_STATE:                                           u32 = 0xc023002f;
pub const SMB_NTSTATUS_NDIS_NOT_SUPPORTED:                                             u32 = 0xc02300bb;
pub const SMB_NTSTATUS_NDIS_OFFLOAD_POLICY:                                            u32 = 0xc023100f;
pub const SMB_NTSTATUS_NDIS_OFFLOAD_CONNECTION_REJECTED:                               u32 = 0xc0231012;
pub const SMB_NTSTATUS_NDIS_OFFLOAD_PATH_REJECTED:                                     u32 = 0xc0231013;
pub const SMB_NTSTATUS_NDIS_DOT11_AUTO_CONFIG_ENABLED:                                 u32 = 0xc0232000;
pub const SMB_NTSTATUS_NDIS_DOT11_MEDIA_IN_USE:                                        u32 = 0xc0232001;
pub const SMB_NTSTATUS_NDIS_DOT11_POWER_STATE_INVALID:                                 u32 = 0xc0232002;
pub const SMB_NTSTATUS_NDIS_PM_WOL_PATTERN_LIST_FULL:                                  u32 = 0xc0232003;
pub const SMB_NTSTATUS_NDIS_PM_PROTOCOL_OFFLOAD_LIST_FULL:                             u32 = 0xc0232004;
pub const SMB_NTSTATUS_IPSEC_BAD_SPI:                                                  u32 = 0xc0360001;
pub const SMB_NTSTATUS_IPSEC_SA_LIFETIME_EXPIRED:                                      u32 = 0xc0360002;
pub const SMB_NTSTATUS_IPSEC_WRONG_SA:                                                 u32 = 0xc0360003;
pub const SMB_NTSTATUS_IPSEC_REPLAY_CHECK_FAILED:                                      u32 = 0xc0360004;
pub const SMB_NTSTATUS_IPSEC_INVALID_PACKET:                                           u32 = 0xc0360005;
pub const SMB_NTSTATUS_IPSEC_INTEGRITY_CHECK_FAILED:                                   u32 = 0xc0360006;
pub const SMB_NTSTATUS_IPSEC_CLEAR_TEXT_DROP:                                          u32 = 0xc0360007;
pub const SMB_NTSTATUS_IPSEC_AUTH_FIREWALL_DROP:                                       u32 = 0xc0360008;
pub const SMB_NTSTATUS_IPSEC_THROTTLE_DROP:                                            u32 = 0xc0360009;
pub const SMB_NTSTATUS_IPSEC_DOSP_BLOCK:                                               u32 = 0xc0368000;
pub const SMB_NTSTATUS_IPSEC_DOSP_RECEIVED_MULTICAST:                                  u32 = 0xc0368001;
pub const SMB_NTSTATUS_IPSEC_DOSP_INVALID_PACKET:                                      u32 = 0xc0368002;
pub const SMB_NTSTATUS_IPSEC_DOSP_STATE_LOOKUP_FAILED:                                 u32 = 0xc0368003;
pub const SMB_NTSTATUS_IPSEC_DOSP_MAX_ENTRIES:                                         u32 = 0xc0368004;
pub const SMB_NTSTATUS_IPSEC_DOSP_KEYMOD_NOT_ALLOWED:                                  u32 = 0xc0368005;
pub const SMB_NTSTATUS_IPSEC_DOSP_MAX_PER_IP_RATELIMIT_QUEUES:                         u32 = 0xc0368006;
pub const SMB_NTSTATUS_VOLMGR_MIRROR_NOT_SUPPORTED:                                    u32 = 0xc038005b;
pub const SMB_NTSTATUS_VOLMGR_RAID5_NOT_SUPPORTED:                                     u32 = 0xc038005c;
pub const SMB_NTSTATUS_VIRTDISK_PROVIDER_NOT_FOUND:                                    u32 = 0xc03a0014;
pub const SMB_NTSTATUS_VIRTDISK_NOT_VIRTUAL_DISK:                                      u32 = 0xc03a0015;
pub const SMB_NTSTATUS_VHD_PARENT_VHD_ACCESS_DENIED:                                   u32 = 0xc03a0016;
pub const SMB_NTSTATUS_VHD_CHILD_PARENT_SIZE_MISMATCH:                                 u32 = 0xc03a0017;
pub const SMB_NTSTATUS_VHD_DIFFERENCING_CHAIN_CYCLE_DETECTED:                          u32 = 0xc03a0018;
pub const SMB_NTSTATUS_VHD_DIFFERENCING_CHAIN_ERROR_IN_PARENT:                         u32 = 0xc03a0019;




pub fn smb_ntstatus_string(c: u32) -> String {
    match c {
        SMB_NTSTATUS_SUCCESS                                                        => "STATUS_SUCCESS",
        SMB_NTSTATUS_WAIT_1                                                         => "STATUS_WAIT_1",
        SMB_NTSTATUS_WAIT_2                                                         => "STATUS_WAIT_2",
        SMB_NTSTATUS_WAIT_3                                                         => "STATUS_WAIT_3",
        SMB_NTSTATUS_WAIT_63                                                        => "STATUS_WAIT_63",
        SMB_NTSTATUS_ABANDONED                                                      => "STATUS_ABANDONED",
        SMB_NTSTATUS_ABANDONED_WAIT_63                                              => "STATUS_ABANDONED_WAIT_63",
        SMB_NTSTATUS_USER_APC                                                       => "STATUS_USER_APC",
        SMB_NTSTATUS_ALERTED                                                        => "STATUS_ALERTED",
        SMB_NTSTATUS_TIMEOUT                                                        => "STATUS_TIMEOUT",
        SMB_NTSTATUS_PENDING                                                        => "STATUS_PENDING",
        SMB_NTSTATUS_REPARSE                                                        => "STATUS_REPARSE",
        SMB_NTSTATUS_MORE_ENTRIES                                                   => "STATUS_MORE_ENTRIES",
        SMB_NTSTATUS_NOT_ALL_ASSIGNED                                               => "STATUS_NOT_ALL_ASSIGNED",
        SMB_NTSTATUS_SOME_NOT_MAPPED                                                => "STATUS_SOME_NOT_MAPPED",
        SMB_NTSTATUS_OPLOCK_BREAK_IN_PROGRESS                                       => "STATUS_OPLOCK_BREAK_IN_PROGRESS",
        SMB_NTSTATUS_VOLUME_MOUNTED                                                 => "STATUS_VOLUME_MOUNTED",
        SMB_NTSTATUS_RXACT_COMMITTED                                                => "STATUS_RXACT_COMMITTED",
        SMB_NTSTATUS_NOTIFY_CLEANUP                                                 => "STATUS_NOTIFY_CLEANUP",
        SMB_NTSTATUS_NOTIFY_ENUM_DIR                                                => "STATUS_NOTIFY_ENUM_DIR",
        SMB_NTSTATUS_NO_QUOTAS_FOR_ACCOUNT                                          => "STATUS_NO_QUOTAS_FOR_ACCOUNT",
        SMB_NTSTATUS_PRIMARY_TRANSPORT_CONNECT_FAILED                               => "STATUS_PRIMARY_TRANSPORT_CONNECT_FAILED",
        SMB_NTSTATUS_PAGE_FAULT_TRANSITION                                          => "STATUS_PAGE_FAULT_TRANSITION",
        SMB_NTSTATUS_PAGE_FAULT_DEMAND_ZERO                                         => "STATUS_PAGE_FAULT_DEMAND_ZERO",
        SMB_NTSTATUS_PAGE_FAULT_COPY_ON_WRITE                                       => "STATUS_PAGE_FAULT_COPY_ON_WRITE",
        SMB_NTSTATUS_PAGE_FAULT_GUARD_PAGE                                          => "STATUS_PAGE_FAULT_GUARD_PAGE",
        SMB_NTSTATUS_PAGE_FAULT_PAGING_FILE                                         => "STATUS_PAGE_FAULT_PAGING_FILE",
        SMB_NTSTATUS_CACHE_PAGE_LOCKED                                              => "STATUS_CACHE_PAGE_LOCKED",
        SMB_NTSTATUS_CRASH_DUMP                                                     => "STATUS_CRASH_DUMP",
        SMB_NTSTATUS_BUFFER_ALL_ZEROS                                               => "STATUS_BUFFER_ALL_ZEROS",
        SMB_NTSTATUS_REPARSE_OBJECT                                                 => "STATUS_REPARSE_OBJECT",
        SMB_NTSTATUS_RESOURCE_REQUIREMENTS_CHANGED                                  => "STATUS_RESOURCE_REQUIREMENTS_CHANGED",
        SMB_NTSTATUS_TRANSLATION_COMPLETE                                           => "STATUS_TRANSLATION_COMPLETE",
        SMB_NTSTATUS_DS_MEMBERSHIP_EVALUATED_LOCALLY                                => "STATUS_DS_MEMBERSHIP_EVALUATED_LOCALLY",
        SMB_NTSTATUS_NOTHING_TO_TERMINATE                                           => "STATUS_NOTHING_TO_TERMINATE",
        SMB_NTSTATUS_PROCESS_NOT_IN_JOB                                             => "STATUS_PROCESS_NOT_IN_JOB",
        SMB_NTSTATUS_PROCESS_IN_JOB                                                 => "STATUS_PROCESS_IN_JOB",
        SMB_NTSTATUS_VOLSNAP_HIBERNATE_READY                                        => "STATUS_VOLSNAP_HIBERNATE_READY",
        SMB_NTSTATUS_FSFILTER_OP_COMPLETED_SUCCESSFULLY                             => "STATUS_FSFILTER_OP_COMPLETED_SUCCESSFULLY",
        SMB_NTSTATUS_INTERRUPT_VECTOR_ALREADY_CONNECTED                             => "STATUS_INTERRUPT_VECTOR_ALREADY_CONNECTED",
        SMB_NTSTATUS_INTERRUPT_STILL_CONNECTED                                      => "STATUS_INTERRUPT_STILL_CONNECTED",
        SMB_NTSTATUS_PROCESS_CLONED                                                 => "STATUS_PROCESS_CLONED",
        SMB_NTSTATUS_FILE_LOCKED_WITH_ONLY_READERS                                  => "STATUS_FILE_LOCKED_WITH_ONLY_READERS",
        SMB_NTSTATUS_FILE_LOCKED_WITH_WRITERS                                       => "STATUS_FILE_LOCKED_WITH_WRITERS",
        SMB_NTSTATUS_RESOURCEMANAGER_READ_ONLY                                      => "STATUS_RESOURCEMANAGER_READ_ONLY",
        SMB_NTSTATUS_WAIT_FOR_OPLOCK                                                => "STATUS_WAIT_FOR_OPLOCK",
        SMB_NTDBG_EXCEPTION_HANDLED                                                 => "DBG_EXCEPTION_HANDLED",
        SMB_NTDBG_CONTINUE                                                          => "DBG_CONTINUE",
        SMB_NTSTATUS_FLT_IO_COMPLETE                                                => "STATUS_FLT_IO_COMPLETE",
        SMB_NTSTATUS_FILE_NOT_AVAILABLE                                             => "STATUS_FILE_NOT_AVAILABLE",
        SMB_NTSTATUS_SHARE_UNAVAILABLE                                              => "STATUS_SHARE_UNAVAILABLE",
        SMB_NTSTATUS_CALLBACK_RETURNED_THREAD_AFFINITY                              => "STATUS_CALLBACK_RETURNED_THREAD_AFFINITY",
        SMB_NTSTATUS_OBJECT_NAME_EXISTS                                             => "STATUS_OBJECT_NAME_EXISTS",
        SMB_NTSTATUS_THREAD_WAS_SUSPENDED                                           => "STATUS_THREAD_WAS_SUSPENDED",
        SMB_NTSTATUS_WORKING_SET_LIMIT_RANGE                                        => "STATUS_WORKING_SET_LIMIT_RANGE",
        SMB_NTSTATUS_IMAGE_NOT_AT_BASE                                              => "STATUS_IMAGE_NOT_AT_BASE",
        SMB_NTSTATUS_RXACT_STATE_CREATED                                            => "STATUS_RXACT_STATE_CREATED",
        SMB_NTSTATUS_SEGMENT_NOTIFICATION                                           => "STATUS_SEGMENT_NOTIFICATION",
        SMB_NTSTATUS_LOCAL_USER_SESSION_KEY                                         => "STATUS_LOCAL_USER_SESSION_KEY",
        SMB_NTSTATUS_BAD_CURRENT_DIRECTORY                                          => "STATUS_BAD_CURRENT_DIRECTORY",
        SMB_NTSTATUS_SERIAL_MORE_WRITES                                             => "STATUS_SERIAL_MORE_WRITES",
        SMB_NTSTATUS_REGISTRY_RECOVERED                                             => "STATUS_REGISTRY_RECOVERED",
        SMB_NTSTATUS_FT_READ_RECOVERY_FROM_BACKUP                                   => "STATUS_FT_READ_RECOVERY_FROM_BACKUP",
        SMB_NTSTATUS_FT_WRITE_RECOVERY                                              => "STATUS_FT_WRITE_RECOVERY",
        SMB_NTSTATUS_SERIAL_COUNTER_TIMEOUT                                         => "STATUS_SERIAL_COUNTER_TIMEOUT",
        SMB_NTSTATUS_NULL_LM_PASSWORD                                               => "STATUS_NULL_LM_PASSWORD",
        SMB_NTSTATUS_IMAGE_MACHINE_TYPE_MISMATCH                                    => "STATUS_IMAGE_MACHINE_TYPE_MISMATCH",
        SMB_NTSTATUS_RECEIVE_PARTIAL                                                => "STATUS_RECEIVE_PARTIAL",
        SMB_NTSTATUS_RECEIVE_EXPEDITED                                              => "STATUS_RECEIVE_EXPEDITED",
        SMB_NTSTATUS_RECEIVE_PARTIAL_EXPEDITED                                      => "STATUS_RECEIVE_PARTIAL_EXPEDITED",
        SMB_NTSTATUS_EVENT_DONE                                                     => "STATUS_EVENT_DONE",
        SMB_NTSTATUS_EVENT_PENDING                                                  => "STATUS_EVENT_PENDING",
        SMB_NTSTATUS_CHECKING_FILE_SYSTEM                                           => "STATUS_CHECKING_FILE_SYSTEM",
        SMB_NTSTATUS_FATAL_APP_EXIT                                                 => "STATUS_FATAL_APP_EXIT",
        SMB_NTSTATUS_PREDEFINED_HANDLE                                              => "STATUS_PREDEFINED_HANDLE",
        SMB_NTSTATUS_WAS_UNLOCKED                                                   => "STATUS_WAS_UNLOCKED",
        SMB_NTSTATUS_SERVICE_NOTIFICATION                                           => "STATUS_SERVICE_NOTIFICATION",
        SMB_NTSTATUS_WAS_LOCKED                                                     => "STATUS_WAS_LOCKED",
        SMB_NTSTATUS_LOG_HARD_ERROR                                                 => "STATUS_LOG_HARD_ERROR",
        SMB_NTSTATUS_ALREADY_WIN32                                                  => "STATUS_ALREADY_WIN32",
        SMB_NTSTATUS_WX86_UNSIMULATE                                                => "STATUS_WX86_UNSIMULATE",
        SMB_NTSTATUS_WX86_CONTINUE                                                  => "STATUS_WX86_CONTINUE",
        SMB_NTSTATUS_WX86_SINGLE_STEP                                               => "STATUS_WX86_SINGLE_STEP",
        SMB_NTSTATUS_WX86_BREAKPOINT                                                => "STATUS_WX86_BREAKPOINT",
        SMB_NTSTATUS_WX86_EXCEPTION_CONTINUE                                        => "STATUS_WX86_EXCEPTION_CONTINUE",
        SMB_NTSTATUS_WX86_EXCEPTION_LASTCHANCE                                      => "STATUS_WX86_EXCEPTION_LASTCHANCE",
        SMB_NTSTATUS_WX86_EXCEPTION_CHAIN                                           => "STATUS_WX86_EXCEPTION_CHAIN",
        SMB_NTSTATUS_IMAGE_MACHINE_TYPE_MISMATCH_EXE                                => "STATUS_IMAGE_MACHINE_TYPE_MISMATCH_EXE",
        SMB_NTSTATUS_NO_YIELD_PERFORMED                                             => "STATUS_NO_YIELD_PERFORMED",
        SMB_NTSTATUS_TIMER_RESUME_IGNORED                                           => "STATUS_TIMER_RESUME_IGNORED",
        SMB_NTSTATUS_ARBITRATION_UNHANDLED                                          => "STATUS_ARBITRATION_UNHANDLED",
        SMB_NTSTATUS_CARDBUS_NOT_SUPPORTED                                          => "STATUS_CARDBUS_NOT_SUPPORTED",
        SMB_NTSTATUS_WX86_CREATEWX86TIB                                             => "STATUS_WX86_CREATEWX86TIB",
        SMB_NTSTATUS_MP_PROCESSOR_MISMATCH                                          => "STATUS_MP_PROCESSOR_MISMATCH",
        SMB_NTSTATUS_HIBERNATED                                                     => "STATUS_HIBERNATED",
        SMB_NTSTATUS_RESUME_HIBERNATION                                             => "STATUS_RESUME_HIBERNATION",
        SMB_NTSTATUS_FIRMWARE_UPDATED                                               => "STATUS_FIRMWARE_UPDATED",
        SMB_NTSTATUS_DRIVERS_LEAKING_LOCKED_PAGES                                   => "STATUS_DRIVERS_LEAKING_LOCKED_PAGES",
        SMB_NTSTATUS_MESSAGE_RETRIEVED                                              => "STATUS_MESSAGE_RETRIEVED",
        SMB_NTSTATUS_SYSTEM_POWERSTATE_TRANSITION                                   => "STATUS_SYSTEM_POWERSTATE_TRANSITION",
        SMB_NTSTATUS_ALPC_CHECK_COMPLETION_LIST                                     => "STATUS_ALPC_CHECK_COMPLETION_LIST",
        SMB_NTSTATUS_SYSTEM_POWERSTATE_COMPLEX_TRANSITION                           => "STATUS_SYSTEM_POWERSTATE_COMPLEX_TRANSITION",
        SMB_NTSTATUS_ACCESS_AUDIT_BY_POLICY                                         => "STATUS_ACCESS_AUDIT_BY_POLICY",
        SMB_NTSTATUS_ABANDON_HIBERFILE                                              => "STATUS_ABANDON_HIBERFILE",
        SMB_NTSTATUS_BIZRULES_NOT_ENABLED                                           => "STATUS_BIZRULES_NOT_ENABLED",
        SMB_NTSTATUS_WAKE_SYSTEM                                                    => "STATUS_WAKE_SYSTEM",
        SMB_NTSTATUS_DS_SHUTTING_DOWN                                               => "STATUS_DS_SHUTTING_DOWN",
        SMB_NTDBG_REPLY_LATER                                                       => "DBG_REPLY_LATER",
        SMB_NTDBG_UNABLE_TO_PROVIDE_HANDLE                                          => "DBG_UNABLE_TO_PROVIDE_HANDLE",
        SMB_NTDBG_TERMINATE_THREAD                                                  => "DBG_TERMINATE_THREAD",
        SMB_NTDBG_TERMINATE_PROCESS                                                 => "DBG_TERMINATE_PROCESS",
        SMB_NTDBG_CONTROL_C                                                         => "DBG_CONTROL_C",
        SMB_NTDBG_PRINTEXCEPTION_C                                                  => "DBG_PRINTEXCEPTION_C",
        SMB_NTDBG_RIPEXCEPTION                                                      => "DBG_RIPEXCEPTION",
        SMB_NTDBG_CONTROL_BREAK                                                     => "DBG_CONTROL_BREAK",
        SMB_NTDBG_COMMAND_EXCEPTION                                                 => "DBG_COMMAND_EXCEPTION",
        SMB_NTRPC_NT_UUID_LOCAL_ONLY                                                => "RPC_NT_UUID_LOCAL_ONLY",
        SMB_NTRPC_NT_SEND_INCOMPLETE                                                => "RPC_NT_SEND_INCOMPLETE",
        SMB_NTSTATUS_CTX_CDM_CONNECT                                                => "STATUS_CTX_CDM_CONNECT",
        SMB_NTSTATUS_CTX_CDM_DISCONNECT                                             => "STATUS_CTX_CDM_DISCONNECT",
        SMB_NTSTATUS_SXS_RELEASE_ACTIVATION_CONTEXT                                 => "STATUS_SXS_RELEASE_ACTIVATION_CONTEXT",
        SMB_NTSTATUS_RECOVERY_NOT_NEEDED                                            => "STATUS_RECOVERY_NOT_NEEDED",
        SMB_NTSTATUS_RM_ALREADY_STARTED                                             => "STATUS_RM_ALREADY_STARTED",
        SMB_NTSTATUS_LOG_NO_RESTART                                                 => "STATUS_LOG_NO_RESTART",
        SMB_NTSTATUS_VIDEO_DRIVER_DEBUG_REPORT_REQUEST                              => "STATUS_VIDEO_DRIVER_DEBUG_REPORT_REQUEST",
        SMB_NTSTATUS_GRAPHICS_PARTIAL_DATA_POPULATED                                => "STATUS_GRAPHICS_PARTIAL_DATA_POPULATED",
        SMB_NTSTATUS_GRAPHICS_DRIVER_MISMATCH                                       => "STATUS_GRAPHICS_DRIVER_MISMATCH",
        SMB_NTSTATUS_GRAPHICS_MODE_NOT_PINNED                                       => "STATUS_GRAPHICS_MODE_NOT_PINNED",
        SMB_NTSTATUS_GRAPHICS_NO_PREFERRED_MODE                                     => "STATUS_GRAPHICS_NO_PREFERRED_MODE",
        SMB_NTSTATUS_GRAPHICS_DATASET_IS_EMPTY                                      => "STATUS_GRAPHICS_DATASET_IS_EMPTY",
        SMB_NTSTATUS_GRAPHICS_NO_MORE_ELEMENTS_IN_DATASET                           => "STATUS_GRAPHICS_NO_MORE_ELEMENTS_IN_DATASET",
        SMB_NTSTATUS_GRAPHICS_PATH_CONTENT_GEOMETRY_TRANSFORMATION_NOT_PINNED       => "STATUS_GRAPHICS_PATH_CONTENT_GEOMETRY_TRANSFORMATION_NOT_PINNED",
        SMB_NTSTATUS_GRAPHICS_UNKNOWN_CHILD_STATUS                                  => "STATUS_GRAPHICS_UNKNOWN_CHILD_STATUS",
        SMB_NTSTATUS_GRAPHICS_LEADLINK_START_DEFERRED                               => "STATUS_GRAPHICS_LEADLINK_START_DEFERRED",
        SMB_NTSTATUS_GRAPHICS_POLLING_TOO_FREQUENTLY                                => "STATUS_GRAPHICS_POLLING_TOO_FREQUENTLY",
        SMB_NTSTATUS_GRAPHICS_START_DEFERRED                                        => "STATUS_GRAPHICS_START_DEFERRED",
        SMB_NTSTATUS_NDIS_INDICATION_REQUIRED                                       => "STATUS_NDIS_INDICATION_REQUIRED",
        SMB_NTSTATUS_GUARD_PAGE_VIOLATION                                           => "STATUS_GUARD_PAGE_VIOLATION",
        SMB_NTSTATUS_DATATYPE_MISALIGNMENT                                          => "STATUS_DATATYPE_MISALIGNMENT",
        SMB_NTSTATUS_BREAKPOINT                                                     => "STATUS_BREAKPOINT",
        SMB_NTSTATUS_SINGLE_STEP                                                    => "STATUS_SINGLE_STEP",
        SMB_NTSTATUS_BUFFER_OVERFLOW                                                => "STATUS_BUFFER_OVERFLOW",
        SMB_NTSTATUS_NO_MORE_FILES                                                  => "STATUS_NO_MORE_FILES",
        SMB_NTSTATUS_WAKE_SYSTEM_DEBUGGER                                           => "STATUS_WAKE_SYSTEM_DEBUGGER",
        SMB_NTSTATUS_HANDLES_CLOSED                                                 => "STATUS_HANDLES_CLOSED",
        SMB_NTSTATUS_NO_INHERITANCE                                                 => "STATUS_NO_INHERITANCE",
        SMB_NTSTATUS_GUID_SUBSTITUTION_MADE                                         => "STATUS_GUID_SUBSTITUTION_MADE",
        SMB_NTSTATUS_PARTIAL_COPY                                                   => "STATUS_PARTIAL_COPY",
        SMB_NTSTATUS_DEVICE_PAPER_EMPTY                                             => "STATUS_DEVICE_PAPER_EMPTY",
        SMB_NTSTATUS_DEVICE_POWERED_OFF                                             => "STATUS_DEVICE_POWERED_OFF",
        SMB_NTSTATUS_DEVICE_OFF_LINE                                                => "STATUS_DEVICE_OFF_LINE",
        SMB_NTSTATUS_DEVICE_BUSY                                                    => "STATUS_DEVICE_BUSY",
        SMB_NTSTATUS_NO_MORE_EAS                                                    => "STATUS_NO_MORE_EAS",
        SMB_NTSTATUS_INVALID_EA_NAME                                                => "STATUS_INVALID_EA_NAME",
        SMB_NTSTATUS_EA_LIST_INCONSISTENT                                           => "STATUS_EA_LIST_INCONSISTENT",
        SMB_NTSTATUS_INVALID_EA_FLAG                                                => "STATUS_INVALID_EA_FLAG",
        SMB_NTSTATUS_VERIFY_REQUIRED                                                => "STATUS_VERIFY_REQUIRED",
        SMB_NTSTATUS_EXTRANEOUS_INFORMATION                                         => "STATUS_EXTRANEOUS_INFORMATION",
        SMB_NTSTATUS_RXACT_COMMIT_NECESSARY                                         => "STATUS_RXACT_COMMIT_NECESSARY",
        SMB_NTSTATUS_NO_MORE_ENTRIES                                                => "STATUS_NO_MORE_ENTRIES",
        SMB_NTSTATUS_FILEMARK_DETECTED                                              => "STATUS_FILEMARK_DETECTED",
        SMB_NTSTATUS_MEDIA_CHANGED                                                  => "STATUS_MEDIA_CHANGED",
        SMB_NTSTATUS_BUS_RESET                                                      => "STATUS_BUS_RESET",
        SMB_NTSTATUS_END_OF_MEDIA                                                   => "STATUS_END_OF_MEDIA",
        SMB_NTSTATUS_BEGINNING_OF_MEDIA                                             => "STATUS_BEGINNING_OF_MEDIA",
        SMB_NTSTATUS_MEDIA_CHECK                                                    => "STATUS_MEDIA_CHECK",
        SMB_NTSTATUS_SETMARK_DETECTED                                               => "STATUS_SETMARK_DETECTED",
        SMB_NTSTATUS_NO_DATA_DETECTED                                               => "STATUS_NO_DATA_DETECTED",
        SMB_NTSTATUS_REDIRECTOR_HAS_OPEN_HANDLES                                    => "STATUS_REDIRECTOR_HAS_OPEN_HANDLES",
        SMB_NTSTATUS_SERVER_HAS_OPEN_HANDLES                                        => "STATUS_SERVER_HAS_OPEN_HANDLES",
        SMB_NTSTATUS_ALREADY_DISCONNECTED                                           => "STATUS_ALREADY_DISCONNECTED",
        SMB_NTSTATUS_LONGJUMP                                                       => "STATUS_LONGJUMP",
        SMB_NTSTATUS_CLEANER_CARTRIDGE_INSTALLED                                    => "STATUS_CLEANER_CARTRIDGE_INSTALLED",
        SMB_NTSTATUS_PLUGPLAY_QUERY_VETOED                                          => "STATUS_PLUGPLAY_QUERY_VETOED",
        SMB_NTSTATUS_UNWIND_CONSOLIDATE                                             => "STATUS_UNWIND_CONSOLIDATE",
        SMB_NTSTATUS_REGISTRY_HIVE_RECOVERED                                        => "STATUS_REGISTRY_HIVE_RECOVERED",
        SMB_NTSTATUS_DLL_MIGHT_BE_INSECURE                                          => "STATUS_DLL_MIGHT_BE_INSECURE",
        SMB_NTSTATUS_DLL_MIGHT_BE_INCOMPATIBLE                                      => "STATUS_DLL_MIGHT_BE_INCOMPATIBLE",
        SMB_NTSTATUS_STOPPED_ON_SYMLINK                                             => "STATUS_STOPPED_ON_SYMLINK",
        SMB_NTSTATUS_DEVICE_REQUIRES_CLEANING                                       => "STATUS_DEVICE_REQUIRES_CLEANING",
        SMB_NTSTATUS_DEVICE_DOOR_OPEN                                               => "STATUS_DEVICE_DOOR_OPEN",
        SMB_NTSTATUS_DATA_LOST_REPAIR                                               => "STATUS_DATA_LOST_REPAIR",
        SMB_NTDBG_EXCEPTION_NOT_HANDLED                                             => "DBG_EXCEPTION_NOT_HANDLED",
        SMB_NTSTATUS_CLUSTER_NODE_ALREADY_UP                                        => "STATUS_CLUSTER_NODE_ALREADY_UP",
        SMB_NTSTATUS_CLUSTER_NODE_ALREADY_DOWN                                      => "STATUS_CLUSTER_NODE_ALREADY_DOWN",
        SMB_NTSTATUS_CLUSTER_NETWORK_ALREADY_ONLINE                                 => "STATUS_CLUSTER_NETWORK_ALREADY_ONLINE",
        SMB_NTSTATUS_CLUSTER_NETWORK_ALREADY_OFFLINE                                => "STATUS_CLUSTER_NETWORK_ALREADY_OFFLINE",
        SMB_NTSTATUS_CLUSTER_NODE_ALREADY_MEMBER                                    => "STATUS_CLUSTER_NODE_ALREADY_MEMBER",
        SMB_NTSTATUS_COULD_NOT_RESIZE_LOG                                           => "STATUS_COULD_NOT_RESIZE_LOG",
        SMB_NTSTATUS_NO_TXF_METADATA                                                => "STATUS_NO_TXF_METADATA",
        SMB_NTSTATUS_CANT_RECOVER_WITH_HANDLE_OPEN                                  => "STATUS_CANT_RECOVER_WITH_HANDLE_OPEN",
        SMB_NTSTATUS_TXF_METADATA_ALREADY_PRESENT                                   => "STATUS_TXF_METADATA_ALREADY_PRESENT",
        SMB_NTSTATUS_TRANSACTION_SCOPE_CALLBACKS_NOT_SET                            => "STATUS_TRANSACTION_SCOPE_CALLBACKS_NOT_SET",
        SMB_NTSTATUS_VIDEO_HUNG_DISPLAY_DRIVER_THREAD_RECOVERED                     => "STATUS_VIDEO_HUNG_DISPLAY_DRIVER_THREAD_RECOVERED",
        SMB_NTSTATUS_FLT_BUFFER_TOO_SMALL                                           => "STATUS_FLT_BUFFER_TOO_SMALL",
        SMB_NTSTATUS_FVE_PARTIAL_METADATA                                           => "STATUS_FVE_PARTIAL_METADATA",
        SMB_NTSTATUS_FVE_TRANSIENT_STATE                                            => "STATUS_FVE_TRANSIENT_STATE",
        SMB_NTSTATUS_UNSUCCESSFUL                                                   => "STATUS_UNSUCCESSFUL",
        SMB_NTSTATUS_NOT_IMPLEMENTED                                                => "STATUS_NOT_IMPLEMENTED",
        SMB_NTSTATUS_INVALID_INFO_CLASS                                             => "STATUS_INVALID_INFO_CLASS",
        SMB_NTSTATUS_INFO_LENGTH_MISMATCH                                           => "STATUS_INFO_LENGTH_MISMATCH",
        SMB_NTSTATUS_ACCESS_VIOLATION                                               => "STATUS_ACCESS_VIOLATION",
        SMB_NTSTATUS_IN_PAGE_ERROR                                                  => "STATUS_IN_PAGE_ERROR",
        SMB_NTSTATUS_PAGEFILE_QUOTA                                                 => "STATUS_PAGEFILE_QUOTA",
        SMB_NTSTATUS_INVALID_HANDLE                                                 => "STATUS_INVALID_HANDLE",
        SMB_NTSTATUS_BAD_INITIAL_STACK                                              => "STATUS_BAD_INITIAL_STACK",
        SMB_NTSTATUS_BAD_INITIAL_PC                                                 => "STATUS_BAD_INITIAL_PC",
        SMB_NTSTATUS_INVALID_CID                                                    => "STATUS_INVALID_CID",
        SMB_NTSTATUS_TIMER_NOT_CANCELED                                             => "STATUS_TIMER_NOT_CANCELED",
        SMB_NTSTATUS_INVALID_PARAMETER                                              => "STATUS_INVALID_PARAMETER",
        SMB_NTSTATUS_NO_SUCH_DEVICE                                                 => "STATUS_NO_SUCH_DEVICE",
        SMB_NTSTATUS_NO_SUCH_FILE                                                   => "STATUS_NO_SUCH_FILE",
        SMB_NTSTATUS_INVALID_DEVICE_REQUEST                                         => "STATUS_INVALID_DEVICE_REQUEST",
        SMB_NTSTATUS_END_OF_FILE                                                    => "STATUS_END_OF_FILE",
        SMB_NTSTATUS_WRONG_VOLUME                                                   => "STATUS_WRONG_VOLUME",
        SMB_NTSTATUS_NO_MEDIA_IN_DEVICE                                             => "STATUS_NO_MEDIA_IN_DEVICE",
        SMB_NTSTATUS_UNRECOGNIZED_MEDIA                                             => "STATUS_UNRECOGNIZED_MEDIA",
        SMB_NTSTATUS_NONEXISTENT_SECTOR                                             => "STATUS_NONEXISTENT_SECTOR",
        SMB_NTSTATUS_MORE_PROCESSING_REQUIRED                                       => "STATUS_MORE_PROCESSING_REQUIRED",
        SMB_NTSTATUS_NO_MEMORY                                                      => "STATUS_NO_MEMORY",
        SMB_NTSTATUS_CONFLICTING_ADDRESSES                                          => "STATUS_CONFLICTING_ADDRESSES",
        SMB_NTSTATUS_NOT_MAPPED_VIEW                                                => "STATUS_NOT_MAPPED_VIEW",
        SMB_NTSTATUS_UNABLE_TO_FREE_VM                                              => "STATUS_UNABLE_TO_FREE_VM",
        SMB_NTSTATUS_UNABLE_TO_DELETE_SECTION                                       => "STATUS_UNABLE_TO_DELETE_SECTION",
        SMB_NTSTATUS_INVALID_SYSTEM_SERVICE                                         => "STATUS_INVALID_SYSTEM_SERVICE",
        SMB_NTSTATUS_ILLEGAL_INSTRUCTION                                            => "STATUS_ILLEGAL_INSTRUCTION",
        SMB_NTSTATUS_INVALID_LOCK_SEQUENCE                                          => "STATUS_INVALID_LOCK_SEQUENCE",
        SMB_NTSTATUS_INVALID_VIEW_SIZE                                              => "STATUS_INVALID_VIEW_SIZE",
        SMB_NTSTATUS_INVALID_FILE_FOR_SECTION                                       => "STATUS_INVALID_FILE_FOR_SECTION",
        SMB_NTSTATUS_ALREADY_COMMITTED                                              => "STATUS_ALREADY_COMMITTED",
        SMB_NTSTATUS_ACCESS_DENIED                                                  => "STATUS_ACCESS_DENIED",
        SMB_NTSTATUS_BUFFER_TOO_SMALL                                               => "STATUS_BUFFER_TOO_SMALL",
        SMB_NTSTATUS_OBJECT_TYPE_MISMATCH                                           => "STATUS_OBJECT_TYPE_MISMATCH",
        SMB_NTSTATUS_NONCONTINUABLE_EXCEPTION                                       => "STATUS_NONCONTINUABLE_EXCEPTION",
        SMB_NTSTATUS_INVALID_DISPOSITION                                            => "STATUS_INVALID_DISPOSITION",
        SMB_NTSTATUS_UNWIND                                                         => "STATUS_UNWIND",
        SMB_NTSTATUS_BAD_STACK                                                      => "STATUS_BAD_STACK",
        SMB_NTSTATUS_INVALID_UNWIND_TARGET                                          => "STATUS_INVALID_UNWIND_TARGET",
        SMB_NTSTATUS_NOT_LOCKED                                                     => "STATUS_NOT_LOCKED",
        SMB_NTSTATUS_PARITY_ERROR                                                   => "STATUS_PARITY_ERROR",
        SMB_NTSTATUS_UNABLE_TO_DECOMMIT_VM                                          => "STATUS_UNABLE_TO_DECOMMIT_VM",
        SMB_NTSTATUS_NOT_COMMITTED                                                  => "STATUS_NOT_COMMITTED",
        SMB_NTSTATUS_INVALID_PORT_ATTRIBUTES                                        => "STATUS_INVALID_PORT_ATTRIBUTES",
        SMB_NTSTATUS_PORT_MESSAGE_TOO_LONG                                          => "STATUS_PORT_MESSAGE_TOO_LONG",
        SMB_NTSTATUS_INVALID_PARAMETER_MIX                                          => "STATUS_INVALID_PARAMETER_MIX",
        SMB_NTSTATUS_INVALID_QUOTA_LOWER                                            => "STATUS_INVALID_QUOTA_LOWER",
        SMB_NTSTATUS_DISK_CORRUPT_ERROR                                             => "STATUS_DISK_CORRUPT_ERROR",
        SMB_NTSTATUS_OBJECT_NAME_INVALID                                            => "STATUS_OBJECT_NAME_INVALID",
        SMB_NTSTATUS_OBJECT_NAME_NOT_FOUND                                          => "STATUS_OBJECT_NAME_NOT_FOUND",
        SMB_NTSTATUS_OBJECT_NAME_COLLISION                                          => "STATUS_OBJECT_NAME_COLLISION",
        SMB_NTSTATUS_PORT_DISCONNECTED                                              => "STATUS_PORT_DISCONNECTED",
        SMB_NTSTATUS_DEVICE_ALREADY_ATTACHED                                        => "STATUS_DEVICE_ALREADY_ATTACHED",
        SMB_NTSTATUS_OBJECT_PATH_INVALID                                            => "STATUS_OBJECT_PATH_INVALID",
        SMB_NTSTATUS_OBJECT_PATH_NOT_FOUND                                          => "STATUS_OBJECT_PATH_NOT_FOUND",
        SMB_NTSTATUS_OBJECT_PATH_SYNTAX_BAD                                         => "STATUS_OBJECT_PATH_SYNTAX_BAD",
        SMB_NTSTATUS_DATA_OVERRUN                                                   => "STATUS_DATA_OVERRUN",
        SMB_NTSTATUS_DATA_LATE_ERROR                                                => "STATUS_DATA_LATE_ERROR",
        SMB_NTSTATUS_DATA_ERROR                                                     => "STATUS_DATA_ERROR",
        SMB_NTSTATUS_CRC_ERROR                                                      => "STATUS_CRC_ERROR",
        SMB_NTSTATUS_SECTION_TOO_BIG                                                => "STATUS_SECTION_TOO_BIG",
        SMB_NTSTATUS_PORT_CONNECTION_REFUSED                                        => "STATUS_PORT_CONNECTION_REFUSED",
        SMB_NTSTATUS_INVALID_PORT_HANDLE                                            => "STATUS_INVALID_PORT_HANDLE",
        SMB_NTSTATUS_SHARING_VIOLATION                                              => "STATUS_SHARING_VIOLATION",
        SMB_NTSTATUS_QUOTA_EXCEEDED                                                 => "STATUS_QUOTA_EXCEEDED",
        SMB_NTSTATUS_INVALID_PAGE_PROTECTION                                        => "STATUS_INVALID_PAGE_PROTECTION",
        SMB_NTSTATUS_MUTANT_NOT_OWNED                                               => "STATUS_MUTANT_NOT_OWNED",
        SMB_NTSTATUS_SEMAPHORE_LIMIT_EXCEEDED                                       => "STATUS_SEMAPHORE_LIMIT_EXCEEDED",
        SMB_NTSTATUS_PORT_ALREADY_SET                                               => "STATUS_PORT_ALREADY_SET",
        SMB_NTSTATUS_SECTION_NOT_IMAGE                                              => "STATUS_SECTION_NOT_IMAGE",
        SMB_NTSTATUS_SUSPEND_COUNT_EXCEEDED                                         => "STATUS_SUSPEND_COUNT_EXCEEDED",
        SMB_NTSTATUS_THREAD_IS_TERMINATING                                          => "STATUS_THREAD_IS_TERMINATING",
        SMB_NTSTATUS_BAD_WORKING_SET_LIMIT                                          => "STATUS_BAD_WORKING_SET_LIMIT",
        SMB_NTSTATUS_INCOMPATIBLE_FILE_MAP                                          => "STATUS_INCOMPATIBLE_FILE_MAP",
        SMB_NTSTATUS_SECTION_PROTECTION                                             => "STATUS_SECTION_PROTECTION",
        SMB_NTSTATUS_EAS_NOT_SUPPORTED                                              => "STATUS_EAS_NOT_SUPPORTED",
        SMB_NTSTATUS_EA_TOO_LARGE                                                   => "STATUS_EA_TOO_LARGE",
        SMB_NTSTATUS_NONEXISTENT_EA_ENTRY                                           => "STATUS_NONEXISTENT_EA_ENTRY",
        SMB_NTSTATUS_NO_EAS_ON_FILE                                                 => "STATUS_NO_EAS_ON_FILE",
        SMB_NTSTATUS_EA_CORRUPT_ERROR                                               => "STATUS_EA_CORRUPT_ERROR",
        SMB_NTSTATUS_FILE_LOCK_CONFLICT                                             => "STATUS_FILE_LOCK_CONFLICT",
        SMB_NTSTATUS_LOCK_NOT_GRANTED                                               => "STATUS_LOCK_NOT_GRANTED",
        SMB_NTSTATUS_DELETE_PENDING                                                 => "STATUS_DELETE_PENDING",
        SMB_NTSTATUS_CTL_FILE_NOT_SUPPORTED                                         => "STATUS_CTL_FILE_NOT_SUPPORTED",
        SMB_NTSTATUS_UNKNOWN_REVISION                                               => "STATUS_UNKNOWN_REVISION",
        SMB_NTSTATUS_REVISION_MISMATCH                                              => "STATUS_REVISION_MISMATCH",
        SMB_NTSTATUS_INVALID_OWNER                                                  => "STATUS_INVALID_OWNER",
        SMB_NTSTATUS_INVALID_PRIMARY_GROUP                                          => "STATUS_INVALID_PRIMARY_GROUP",
        SMB_NTSTATUS_NO_IMPERSONATION_TOKEN                                         => "STATUS_NO_IMPERSONATION_TOKEN",
        SMB_NTSTATUS_CANT_DISABLE_MANDATORY                                         => "STATUS_CANT_DISABLE_MANDATORY",
        SMB_NTSTATUS_NO_LOGON_SERVERS                                               => "STATUS_NO_LOGON_SERVERS",
        SMB_NTSTATUS_NO_SUCH_LOGON_SESSION                                          => "STATUS_NO_SUCH_LOGON_SESSION",
        SMB_NTSTATUS_NO_SUCH_PRIVILEGE                                              => "STATUS_NO_SUCH_PRIVILEGE",
        SMB_NTSTATUS_PRIVILEGE_NOT_HELD                                             => "STATUS_PRIVILEGE_NOT_HELD",
        SMB_NTSTATUS_INVALID_ACCOUNT_NAME                                           => "STATUS_INVALID_ACCOUNT_NAME",
        SMB_NTSTATUS_USER_EXISTS                                                    => "STATUS_USER_EXISTS",
        SMB_NTSTATUS_NO_SUCH_USER                                                   => "STATUS_NO_SUCH_USER",
        SMB_NTSTATUS_GROUP_EXISTS                                                   => "STATUS_GROUP_EXISTS",
        SMB_NTSTATUS_NO_SUCH_GROUP                                                  => "STATUS_NO_SUCH_GROUP",
        SMB_NTSTATUS_MEMBER_IN_GROUP                                                => "STATUS_MEMBER_IN_GROUP",
        SMB_NTSTATUS_MEMBER_NOT_IN_GROUP                                            => "STATUS_MEMBER_NOT_IN_GROUP",
        SMB_NTSTATUS_LAST_ADMIN                                                     => "STATUS_LAST_ADMIN",
        SMB_NTSTATUS_WRONG_PASSWORD                                                 => "STATUS_WRONG_PASSWORD",
        SMB_NTSTATUS_ILL_FORMED_PASSWORD                                            => "STATUS_ILL_FORMED_PASSWORD",
        SMB_NTSTATUS_PASSWORD_RESTRICTION                                           => "STATUS_PASSWORD_RESTRICTION",
        SMB_NTSTATUS_LOGON_FAILURE                                                  => "STATUS_LOGON_FAILURE",
        SMB_NTSTATUS_ACCOUNT_RESTRICTION                                            => "STATUS_ACCOUNT_RESTRICTION",
        SMB_NTSTATUS_INVALID_LOGON_HOURS                                            => "STATUS_INVALID_LOGON_HOURS",
        SMB_NTSTATUS_INVALID_WORKSTATION                                            => "STATUS_INVALID_WORKSTATION",
        SMB_NTSTATUS_PASSWORD_EXPIRED                                               => "STATUS_PASSWORD_EXPIRED",
        SMB_NTSTATUS_ACCOUNT_DISABLED                                               => "STATUS_ACCOUNT_DISABLED",
        SMB_NTSTATUS_NONE_MAPPED                                                    => "STATUS_NONE_MAPPED",
        SMB_NTSTATUS_TOO_MANY_LUIDS_REQUESTED                                       => "STATUS_TOO_MANY_LUIDS_REQUESTED",
        SMB_NTSTATUS_LUIDS_EXHAUSTED                                                => "STATUS_LUIDS_EXHAUSTED",
        SMB_NTSTATUS_INVALID_SUB_AUTHORITY                                          => "STATUS_INVALID_SUB_AUTHORITY",
        SMB_NTSTATUS_INVALID_ACL                                                    => "STATUS_INVALID_ACL",
        SMB_NTSTATUS_INVALID_SID                                                    => "STATUS_INVALID_SID",
        SMB_NTSTATUS_INVALID_SECURITY_DESCR                                         => "STATUS_INVALID_SECURITY_DESCR",
        SMB_NTSTATUS_PROCEDURE_NOT_FOUND                                            => "STATUS_PROCEDURE_NOT_FOUND",
        SMB_NTSTATUS_INVALID_IMAGE_FORMAT                                           => "STATUS_INVALID_IMAGE_FORMAT",
        SMB_NTSTATUS_NO_TOKEN                                                       => "STATUS_NO_TOKEN",
        SMB_NTSTATUS_BAD_INHERITANCE_ACL                                            => "STATUS_BAD_INHERITANCE_ACL",
        SMB_NTSTATUS_RANGE_NOT_LOCKED                                               => "STATUS_RANGE_NOT_LOCKED",
        SMB_NTSTATUS_DISK_FULL                                                      => "STATUS_DISK_FULL",
        SMB_NTSTATUS_SERVER_DISABLED                                                => "STATUS_SERVER_DISABLED",
        SMB_NTSTATUS_SERVER_NOT_DISABLED                                            => "STATUS_SERVER_NOT_DISABLED",
        SMB_NTSTATUS_TOO_MANY_GUIDS_REQUESTED                                       => "STATUS_TOO_MANY_GUIDS_REQUESTED",
        SMB_NTSTATUS_GUIDS_EXHAUSTED                                                => "STATUS_GUIDS_EXHAUSTED",
        SMB_NTSTATUS_INVALID_ID_AUTHORITY                                           => "STATUS_INVALID_ID_AUTHORITY",
        SMB_NTSTATUS_AGENTS_EXHAUSTED                                               => "STATUS_AGENTS_EXHAUSTED",
        SMB_NTSTATUS_INVALID_VOLUME_LABEL                                           => "STATUS_INVALID_VOLUME_LABEL",
        SMB_NTSTATUS_SECTION_NOT_EXTENDED                                           => "STATUS_SECTION_NOT_EXTENDED",
        SMB_NTSTATUS_NOT_MAPPED_DATA                                                => "STATUS_NOT_MAPPED_DATA",
        SMB_NTSTATUS_RESOURCE_DATA_NOT_FOUND                                        => "STATUS_RESOURCE_DATA_NOT_FOUND",
        SMB_NTSTATUS_RESOURCE_TYPE_NOT_FOUND                                        => "STATUS_RESOURCE_TYPE_NOT_FOUND",
        SMB_NTSTATUS_RESOURCE_NAME_NOT_FOUND                                        => "STATUS_RESOURCE_NAME_NOT_FOUND",
        SMB_NTSTATUS_ARRAY_BOUNDS_EXCEEDED                                          => "STATUS_ARRAY_BOUNDS_EXCEEDED",
        SMB_NTSTATUS_FLOAT_DENORMAL_OPERAND                                         => "STATUS_FLOAT_DENORMAL_OPERAND",
        SMB_NTSTATUS_FLOAT_DIVIDE_BY_ZERO                                           => "STATUS_FLOAT_DIVIDE_BY_ZERO",
        SMB_NTSTATUS_FLOAT_INEXACT_RESULT                                           => "STATUS_FLOAT_INEXACT_RESULT",
        SMB_NTSTATUS_FLOAT_INVALID_OPERATION                                        => "STATUS_FLOAT_INVALID_OPERATION",
        SMB_NTSTATUS_FLOAT_OVERFLOW                                                 => "STATUS_FLOAT_OVERFLOW",
        SMB_NTSTATUS_FLOAT_STACK_CHECK                                              => "STATUS_FLOAT_STACK_CHECK",
        SMB_NTSTATUS_FLOAT_UNDERFLOW                                                => "STATUS_FLOAT_UNDERFLOW",
        SMB_NTSTATUS_INTEGER_DIVIDE_BY_ZERO                                         => "STATUS_INTEGER_DIVIDE_BY_ZERO",
        SMB_NTSTATUS_INTEGER_OVERFLOW                                               => "STATUS_INTEGER_OVERFLOW",
        SMB_NTSTATUS_PRIVILEGED_INSTRUCTION                                         => "STATUS_PRIVILEGED_INSTRUCTION",
        SMB_NTSTATUS_TOO_MANY_PAGING_FILES                                          => "STATUS_TOO_MANY_PAGING_FILES",
        SMB_NTSTATUS_FILE_INVALID                                                   => "STATUS_FILE_INVALID",
        SMB_NTSTATUS_ALLOTTED_SPACE_EXCEEDED                                        => "STATUS_ALLOTTED_SPACE_EXCEEDED",
        SMB_NTSTATUS_INSUFFICIENT_RESOURCES                                         => "STATUS_INSUFFICIENT_RESOURCES",
        SMB_NTSTATUS_DFS_EXIT_PATH_FOUND                                            => "STATUS_DFS_EXIT_PATH_FOUND",
        SMB_NTSTATUS_DEVICE_DATA_ERROR                                              => "STATUS_DEVICE_DATA_ERROR",
        SMB_NTSTATUS_DEVICE_NOT_CONNECTED                                           => "STATUS_DEVICE_NOT_CONNECTED",
        SMB_NTSTATUS_FREE_VM_NOT_AT_BASE                                            => "STATUS_FREE_VM_NOT_AT_BASE",
        SMB_NTSTATUS_MEMORY_NOT_ALLOCATED                                           => "STATUS_MEMORY_NOT_ALLOCATED",
        SMB_NTSTATUS_WORKING_SET_QUOTA                                              => "STATUS_WORKING_SET_QUOTA",
        SMB_NTSTATUS_MEDIA_WRITE_PROTECTED                                          => "STATUS_MEDIA_WRITE_PROTECTED",
        SMB_NTSTATUS_DEVICE_NOT_READY                                               => "STATUS_DEVICE_NOT_READY",
        SMB_NTSTATUS_INVALID_GROUP_ATTRIBUTES                                       => "STATUS_INVALID_GROUP_ATTRIBUTES",
        SMB_NTSTATUS_BAD_IMPERSONATION_LEVEL                                        => "STATUS_BAD_IMPERSONATION_LEVEL",
        SMB_NTSTATUS_CANT_OPEN_ANONYMOUS                                            => "STATUS_CANT_OPEN_ANONYMOUS",
        SMB_NTSTATUS_BAD_VALIDATION_CLASS                                           => "STATUS_BAD_VALIDATION_CLASS",
        SMB_NTSTATUS_BAD_TOKEN_TYPE                                                 => "STATUS_BAD_TOKEN_TYPE",
        SMB_NTSTATUS_BAD_MASTER_BOOT_RECORD                                         => "STATUS_BAD_MASTER_BOOT_RECORD",
        SMB_NTSTATUS_INSTRUCTION_MISALIGNMENT                                       => "STATUS_INSTRUCTION_MISALIGNMENT",
        SMB_NTSTATUS_INSTANCE_NOT_AVAILABLE                                         => "STATUS_INSTANCE_NOT_AVAILABLE",
        SMB_NTSTATUS_PIPE_NOT_AVAILABLE                                             => "STATUS_PIPE_NOT_AVAILABLE",
        SMB_NTSTATUS_INVALID_PIPE_STATE                                             => "STATUS_INVALID_PIPE_STATE",
        SMB_NTSTATUS_PIPE_BUSY                                                      => "STATUS_PIPE_BUSY",
        SMB_NTSTATUS_ILLEGAL_FUNCTION                                               => "STATUS_ILLEGAL_FUNCTION",
        SMB_NTSTATUS_PIPE_DISCONNECTED                                              => "STATUS_PIPE_DISCONNECTED",
        SMB_NTSTATUS_PIPE_CLOSING                                                   => "STATUS_PIPE_CLOSING",
        SMB_NTSTATUS_PIPE_CONNECTED                                                 => "STATUS_PIPE_CONNECTED",
        SMB_NTSTATUS_PIPE_LISTENING                                                 => "STATUS_PIPE_LISTENING",
        SMB_NTSTATUS_INVALID_READ_MODE                                              => "STATUS_INVALID_READ_MODE",
        SMB_NTSTATUS_IO_TIMEOUT                                                     => "STATUS_IO_TIMEOUT",
        SMB_NTSTATUS_FILE_FORCED_CLOSED                                             => "STATUS_FILE_FORCED_CLOSED",
        SMB_NTSTATUS_PROFILING_NOT_STARTED                                          => "STATUS_PROFILING_NOT_STARTED",
        SMB_NTSTATUS_PROFILING_NOT_STOPPED                                          => "STATUS_PROFILING_NOT_STOPPED",
        SMB_NTSTATUS_COULD_NOT_INTERPRET                                            => "STATUS_COULD_NOT_INTERPRET",
        SMB_NTSTATUS_FILE_IS_A_DIRECTORY                                            => "STATUS_FILE_IS_A_DIRECTORY",
        SMB_NTSTATUS_NOT_SUPPORTED                                                  => "STATUS_NOT_SUPPORTED",
        SMB_NTSTATUS_REMOTE_NOT_LISTENING                                           => "STATUS_REMOTE_NOT_LISTENING",
        SMB_NTSTATUS_DUPLICATE_NAME                                                 => "STATUS_DUPLICATE_NAME",
        SMB_NTSTATUS_BAD_NETWORK_PATH                                               => "STATUS_BAD_NETWORK_PATH",
        SMB_NTSTATUS_NETWORK_BUSY                                                   => "STATUS_NETWORK_BUSY",
        SMB_NTSTATUS_DEVICE_DOES_NOT_EXIST                                          => "STATUS_DEVICE_DOES_NOT_EXIST",
        SMB_NTSTATUS_TOO_MANY_COMMANDS                                              => "STATUS_TOO_MANY_COMMANDS",
        SMB_NTSTATUS_ADAPTER_HARDWARE_ERROR                                         => "STATUS_ADAPTER_HARDWARE_ERROR",
        SMB_NTSTATUS_INVALID_NETWORK_RESPONSE                                       => "STATUS_INVALID_NETWORK_RESPONSE",
        SMB_NTSTATUS_UNEXPECTED_NETWORK_ERROR                                       => "STATUS_UNEXPECTED_NETWORK_ERROR",
        SMB_NTSTATUS_BAD_REMOTE_ADAPTER                                             => "STATUS_BAD_REMOTE_ADAPTER",
        SMB_NTSTATUS_PRINT_QUEUE_FULL                                               => "STATUS_PRINT_QUEUE_FULL",
        SMB_NTSTATUS_NO_SPOOL_SPACE                                                 => "STATUS_NO_SPOOL_SPACE",
        SMB_NTSTATUS_PRINT_CANCELLED                                                => "STATUS_PRINT_CANCELLED",
        SMB_NTSTATUS_NETWORK_NAME_DELETED                                           => "STATUS_NETWORK_NAME_DELETED",
        SMB_NTSTATUS_NETWORK_ACCESS_DENIED                                          => "STATUS_NETWORK_ACCESS_DENIED",
        SMB_NTSTATUS_BAD_DEVICE_TYPE                                                => "STATUS_BAD_DEVICE_TYPE",
        SMB_NTSTATUS_BAD_NETWORK_NAME                                               => "STATUS_BAD_NETWORK_NAME",
        SMB_NTSTATUS_TOO_MANY_NAMES                                                 => "STATUS_TOO_MANY_NAMES",
        SMB_NTSTATUS_TOO_MANY_SESSIONS                                              => "STATUS_TOO_MANY_SESSIONS",
        SMB_NTSTATUS_SHARING_PAUSED                                                 => "STATUS_SHARING_PAUSED",
        SMB_NTSTATUS_REQUEST_NOT_ACCEPTED                                           => "STATUS_REQUEST_NOT_ACCEPTED",
        SMB_NTSTATUS_REDIRECTOR_PAUSED                                              => "STATUS_REDIRECTOR_PAUSED",
        SMB_NTSTATUS_NET_WRITE_FAULT                                                => "STATUS_NET_WRITE_FAULT",
        SMB_NTSTATUS_PROFILING_AT_LIMIT                                             => "STATUS_PROFILING_AT_LIMIT",
        SMB_NTSTATUS_NOT_SAME_DEVICE                                                => "STATUS_NOT_SAME_DEVICE",
        SMB_NTSTATUS_FILE_RENAMED                                                   => "STATUS_FILE_RENAMED",
        SMB_NTSTATUS_VIRTUAL_CIRCUIT_CLOSED                                         => "STATUS_VIRTUAL_CIRCUIT_CLOSED",
        SMB_NTSTATUS_NO_SECURITY_ON_OBJECT                                          => "STATUS_NO_SECURITY_ON_OBJECT",
        SMB_NTSTATUS_CANT_WAIT                                                      => "STATUS_CANT_WAIT",
        SMB_NTSTATUS_PIPE_EMPTY                                                     => "STATUS_PIPE_EMPTY",
        SMB_NTSTATUS_CANT_ACCESS_DOMAIN_INFO                                        => "STATUS_CANT_ACCESS_DOMAIN_INFO",
        SMB_NTSTATUS_CANT_TERMINATE_SELF                                            => "STATUS_CANT_TERMINATE_SELF",
        SMB_NTSTATUS_INVALID_SERVER_STATE                                           => "STATUS_INVALID_SERVER_STATE",
        SMB_NTSTATUS_INVALID_DOMAIN_STATE                                           => "STATUS_INVALID_DOMAIN_STATE",
        SMB_NTSTATUS_INVALID_DOMAIN_ROLE                                            => "STATUS_INVALID_DOMAIN_ROLE",
        SMB_NTSTATUS_NO_SUCH_DOMAIN                                                 => "STATUS_NO_SUCH_DOMAIN",
        SMB_NTSTATUS_DOMAIN_EXISTS                                                  => "STATUS_DOMAIN_EXISTS",
        SMB_NTSTATUS_DOMAIN_LIMIT_EXCEEDED                                          => "STATUS_DOMAIN_LIMIT_EXCEEDED",
        SMB_NTSTATUS_OPLOCK_NOT_GRANTED                                             => "STATUS_OPLOCK_NOT_GRANTED",
        SMB_NTSTATUS_INVALID_OPLOCK_PROTOCOL                                        => "STATUS_INVALID_OPLOCK_PROTOCOL",
        SMB_NTSTATUS_INTERNAL_DB_CORRUPTION                                         => "STATUS_INTERNAL_DB_CORRUPTION",
        SMB_NTSTATUS_INTERNAL_ERROR                                                 => "STATUS_INTERNAL_ERROR",
        SMB_NTSTATUS_GENERIC_NOT_MAPPED                                             => "STATUS_GENERIC_NOT_MAPPED",
        SMB_NTSTATUS_BAD_DESCRIPTOR_FORMAT                                          => "STATUS_BAD_DESCRIPTOR_FORMAT",
        SMB_NTSTATUS_INVALID_USER_BUFFER                                            => "STATUS_INVALID_USER_BUFFER",
        SMB_NTSTATUS_UNEXPECTED_IO_ERROR                                            => "STATUS_UNEXPECTED_IO_ERROR",
        SMB_NTSTATUS_UNEXPECTED_MM_CREATE_ERR                                       => "STATUS_UNEXPECTED_MM_CREATE_ERR",
        SMB_NTSTATUS_UNEXPECTED_MM_MAP_ERROR                                        => "STATUS_UNEXPECTED_MM_MAP_ERROR",
        SMB_NTSTATUS_UNEXPECTED_MM_EXTEND_ERR                                       => "STATUS_UNEXPECTED_MM_EXTEND_ERR",
        SMB_NTSTATUS_NOT_LOGON_PROCESS                                              => "STATUS_NOT_LOGON_PROCESS",
        SMB_NTSTATUS_LOGON_SESSION_EXISTS                                           => "STATUS_LOGON_SESSION_EXISTS",
        SMB_NTSTATUS_INVALID_PARAMETER_1                                            => "STATUS_INVALID_PARAMETER_1",
        SMB_NTSTATUS_INVALID_PARAMETER_2                                            => "STATUS_INVALID_PARAMETER_2",
        SMB_NTSTATUS_INVALID_PARAMETER_3                                            => "STATUS_INVALID_PARAMETER_3",
        SMB_NTSTATUS_INVALID_PARAMETER_4                                            => "STATUS_INVALID_PARAMETER_4",
        SMB_NTSTATUS_INVALID_PARAMETER_5                                            => "STATUS_INVALID_PARAMETER_5",
        SMB_NTSTATUS_INVALID_PARAMETER_6                                            => "STATUS_INVALID_PARAMETER_6",
        SMB_NTSTATUS_INVALID_PARAMETER_7                                            => "STATUS_INVALID_PARAMETER_7",
        SMB_NTSTATUS_INVALID_PARAMETER_8                                            => "STATUS_INVALID_PARAMETER_8",
        SMB_NTSTATUS_INVALID_PARAMETER_9                                            => "STATUS_INVALID_PARAMETER_9",
        SMB_NTSTATUS_INVALID_PARAMETER_10                                           => "STATUS_INVALID_PARAMETER_10",
        SMB_NTSTATUS_INVALID_PARAMETER_11                                           => "STATUS_INVALID_PARAMETER_11",
        SMB_NTSTATUS_INVALID_PARAMETER_12                                           => "STATUS_INVALID_PARAMETER_12",
        SMB_NTSTATUS_REDIRECTOR_NOT_STARTED                                         => "STATUS_REDIRECTOR_NOT_STARTED",
        SMB_NTSTATUS_REDIRECTOR_STARTED                                             => "STATUS_REDIRECTOR_STARTED",
        SMB_NTSTATUS_STACK_OVERFLOW                                                 => "STATUS_STACK_OVERFLOW",
        SMB_NTSTATUS_NO_SUCH_PACKAGE                                                => "STATUS_NO_SUCH_PACKAGE",
        SMB_NTSTATUS_BAD_FUNCTION_TABLE                                             => "STATUS_BAD_FUNCTION_TABLE",
        SMB_NTSTATUS_VARIABLE_NOT_FOUND                                             => "STATUS_VARIABLE_NOT_FOUND",
        SMB_NTSTATUS_DIRECTORY_NOT_EMPTY                                            => "STATUS_DIRECTORY_NOT_EMPTY",
        SMB_NTSTATUS_FILE_CORRUPT_ERROR                                             => "STATUS_FILE_CORRUPT_ERROR",
        SMB_NTSTATUS_NOT_A_DIRECTORY                                                => "STATUS_NOT_A_DIRECTORY",
        SMB_NTSTATUS_BAD_LOGON_SESSION_STATE                                        => "STATUS_BAD_LOGON_SESSION_STATE",
        SMB_NTSTATUS_LOGON_SESSION_COLLISION                                        => "STATUS_LOGON_SESSION_COLLISION",
        SMB_NTSTATUS_NAME_TOO_LONG                                                  => "STATUS_NAME_TOO_LONG",
        SMB_NTSTATUS_FILES_OPEN                                                     => "STATUS_FILES_OPEN",
        SMB_NTSTATUS_CONNECTION_IN_USE                                              => "STATUS_CONNECTION_IN_USE",
        SMB_NTSTATUS_MESSAGE_NOT_FOUND                                              => "STATUS_MESSAGE_NOT_FOUND",
        SMB_NTSTATUS_PROCESS_IS_TERMINATING                                         => "STATUS_PROCESS_IS_TERMINATING",
        SMB_NTSTATUS_INVALID_LOGON_TYPE                                             => "STATUS_INVALID_LOGON_TYPE",
        SMB_NTSTATUS_NO_GUID_TRANSLATION                                            => "STATUS_NO_GUID_TRANSLATION",
        SMB_NTSTATUS_CANNOT_IMPERSONATE                                             => "STATUS_CANNOT_IMPERSONATE",
        SMB_NTSTATUS_IMAGE_ALREADY_LOADED                                           => "STATUS_IMAGE_ALREADY_LOADED",
        SMB_NTSTATUS_NO_LDT                                                         => "STATUS_NO_LDT",
        SMB_NTSTATUS_INVALID_LDT_SIZE                                               => "STATUS_INVALID_LDT_SIZE",
        SMB_NTSTATUS_INVALID_LDT_OFFSET                                             => "STATUS_INVALID_LDT_OFFSET",
        SMB_NTSTATUS_INVALID_LDT_DESCRIPTOR                                         => "STATUS_INVALID_LDT_DESCRIPTOR",
        SMB_NTSTATUS_INVALID_IMAGE_NE_FORMAT                                        => "STATUS_INVALID_IMAGE_NE_FORMAT",
        SMB_NTSTATUS_RXACT_INVALID_STATE                                            => "STATUS_RXACT_INVALID_STATE",
        SMB_NTSTATUS_RXACT_COMMIT_FAILURE                                           => "STATUS_RXACT_COMMIT_FAILURE",
        SMB_NTSTATUS_MAPPED_FILE_SIZE_ZERO                                          => "STATUS_MAPPED_FILE_SIZE_ZERO",
        SMB_NTSTATUS_TOO_MANY_OPENED_FILES                                          => "STATUS_TOO_MANY_OPENED_FILES",
        SMB_NTSTATUS_CANCELLED                                                      => "STATUS_CANCELLED",
        SMB_NTSTATUS_CANNOT_DELETE                                                  => "STATUS_CANNOT_DELETE",
        SMB_NTSTATUS_INVALID_COMPUTER_NAME                                          => "STATUS_INVALID_COMPUTER_NAME",
        SMB_NTSTATUS_FILE_DELETED                                                   => "STATUS_FILE_DELETED",
        SMB_NTSTATUS_SPECIAL_ACCOUNT                                                => "STATUS_SPECIAL_ACCOUNT",
        SMB_NTSTATUS_SPECIAL_GROUP                                                  => "STATUS_SPECIAL_GROUP",
        SMB_NTSTATUS_SPECIAL_USER                                                   => "STATUS_SPECIAL_USER",
        SMB_NTSTATUS_MEMBERS_PRIMARY_GROUP                                          => "STATUS_MEMBERS_PRIMARY_GROUP",
        SMB_NTSTATUS_FILE_CLOSED                                                    => "STATUS_FILE_CLOSED",
        SMB_NTSTATUS_TOO_MANY_THREADS                                               => "STATUS_TOO_MANY_THREADS",
        SMB_NTSTATUS_THREAD_NOT_IN_PROCESS                                          => "STATUS_THREAD_NOT_IN_PROCESS",
        SMB_NTSTATUS_TOKEN_ALREADY_IN_USE                                           => "STATUS_TOKEN_ALREADY_IN_USE",
        SMB_NTSTATUS_PAGEFILE_QUOTA_EXCEEDED                                        => "STATUS_PAGEFILE_QUOTA_EXCEEDED",
        SMB_NTSTATUS_COMMITMENT_LIMIT                                               => "STATUS_COMMITMENT_LIMIT",
        SMB_NTSTATUS_INVALID_IMAGE_LE_FORMAT                                        => "STATUS_INVALID_IMAGE_LE_FORMAT",
        SMB_NTSTATUS_INVALID_IMAGE_NOT_MZ                                           => "STATUS_INVALID_IMAGE_NOT_MZ",
        SMB_NTSTATUS_INVALID_IMAGE_PROTECT                                          => "STATUS_INVALID_IMAGE_PROTECT",
        SMB_NTSTATUS_INVALID_IMAGE_WIN_16                                           => "STATUS_INVALID_IMAGE_WIN_16",
        SMB_NTSTATUS_LOGON_SERVER_CONFLICT                                          => "STATUS_LOGON_SERVER_CONFLICT",
        SMB_NTSTATUS_TIME_DIFFERENCE_AT_DC                                          => "STATUS_TIME_DIFFERENCE_AT_DC",
        SMB_NTSTATUS_SYNCHRONIZATION_REQUIRED                                       => "STATUS_SYNCHRONIZATION_REQUIRED",
        SMB_NTSTATUS_DLL_NOT_FOUND                                                  => "STATUS_DLL_NOT_FOUND",
        SMB_NTSTATUS_OPEN_FAILED                                                    => "STATUS_OPEN_FAILED",
        SMB_NTSTATUS_IO_PRIVILEGE_FAILED                                            => "STATUS_IO_PRIVILEGE_FAILED",
        SMB_NTSTATUS_ORDINAL_NOT_FOUND                                              => "STATUS_ORDINAL_NOT_FOUND",
        SMB_NTSTATUS_ENTRYPOINT_NOT_FOUND                                           => "STATUS_ENTRYPOINT_NOT_FOUND",
        SMB_NTSTATUS_CONTROL_C_EXIT                                                 => "STATUS_CONTROL_C_EXIT",
        SMB_NTSTATUS_LOCAL_DISCONNECT                                               => "STATUS_LOCAL_DISCONNECT",
        SMB_NTSTATUS_REMOTE_DISCONNECT                                              => "STATUS_REMOTE_DISCONNECT",
        SMB_NTSTATUS_REMOTE_RESOURCES                                               => "STATUS_REMOTE_RESOURCES",
        SMB_NTSTATUS_LINK_FAILED                                                    => "STATUS_LINK_FAILED",
        SMB_NTSTATUS_LINK_TIMEOUT                                                   => "STATUS_LINK_TIMEOUT",
        SMB_NTSTATUS_INVALID_CONNECTION                                             => "STATUS_INVALID_CONNECTION",
        SMB_NTSTATUS_INVALID_ADDRESS                                                => "STATUS_INVALID_ADDRESS",
        SMB_NTSTATUS_DLL_INIT_FAILED                                                => "STATUS_DLL_INIT_FAILED",
        SMB_NTSTATUS_MISSING_SYSTEMFILE                                             => "STATUS_MISSING_SYSTEMFILE",
        SMB_NTSTATUS_UNHANDLED_EXCEPTION                                            => "STATUS_UNHANDLED_EXCEPTION",
        SMB_NTSTATUS_APP_INIT_FAILURE                                               => "STATUS_APP_INIT_FAILURE",
        SMB_NTSTATUS_PAGEFILE_CREATE_FAILED                                         => "STATUS_PAGEFILE_CREATE_FAILED",
        SMB_NTSTATUS_NO_PAGEFILE                                                    => "STATUS_NO_PAGEFILE",
        SMB_NTSTATUS_INVALID_LEVEL                                                  => "STATUS_INVALID_LEVEL",
        SMB_NTSTATUS_WRONG_PASSWORD_CORE                                            => "STATUS_WRONG_PASSWORD_CORE",
        SMB_NTSTATUS_ILLEGAL_FLOAT_CONTEXT                                          => "STATUS_ILLEGAL_FLOAT_CONTEXT",
        SMB_NTSTATUS_PIPE_BROKEN                                                    => "STATUS_PIPE_BROKEN",
        SMB_NTSTATUS_REGISTRY_CORRUPT                                               => "STATUS_REGISTRY_CORRUPT",
        SMB_NTSTATUS_REGISTRY_IO_FAILED                                             => "STATUS_REGISTRY_IO_FAILED",
        SMB_NTSTATUS_NO_EVENT_PAIR                                                  => "STATUS_NO_EVENT_PAIR",
        SMB_NTSTATUS_UNRECOGNIZED_VOLUME                                            => "STATUS_UNRECOGNIZED_VOLUME",
        SMB_NTSTATUS_SERIAL_NO_DEVICE_INITED                                        => "STATUS_SERIAL_NO_DEVICE_INITED",
        SMB_NTSTATUS_NO_SUCH_ALIAS                                                  => "STATUS_NO_SUCH_ALIAS",
        SMB_NTSTATUS_MEMBER_NOT_IN_ALIAS                                            => "STATUS_MEMBER_NOT_IN_ALIAS",
        SMB_NTSTATUS_MEMBER_IN_ALIAS                                                => "STATUS_MEMBER_IN_ALIAS",
        SMB_NTSTATUS_ALIAS_EXISTS                                                   => "STATUS_ALIAS_EXISTS",
        SMB_NTSTATUS_LOGON_NOT_GRANTED                                              => "STATUS_LOGON_NOT_GRANTED",
        SMB_NTSTATUS_TOO_MANY_SECRETS                                               => "STATUS_TOO_MANY_SECRETS",
        SMB_NTSTATUS_SECRET_TOO_LONG                                                => "STATUS_SECRET_TOO_LONG",
        SMB_NTSTATUS_INTERNAL_DB_ERROR                                              => "STATUS_INTERNAL_DB_ERROR",
        SMB_NTSTATUS_FULLSCREEN_MODE                                                => "STATUS_FULLSCREEN_MODE",
        SMB_NTSTATUS_TOO_MANY_CONTEXT_IDS                                           => "STATUS_TOO_MANY_CONTEXT_IDS",
        SMB_NTSTATUS_LOGON_TYPE_NOT_GRANTED                                         => "STATUS_LOGON_TYPE_NOT_GRANTED",
        SMB_NTSTATUS_NOT_REGISTRY_FILE                                              => "STATUS_NOT_REGISTRY_FILE",
        SMB_NTSTATUS_NT_CROSS_ENCRYPTION_REQUIRED                                   => "STATUS_NT_CROSS_ENCRYPTION_REQUIRED",
        SMB_NTSTATUS_DOMAIN_CTRLR_CONFIG_ERROR                                      => "STATUS_DOMAIN_CTRLR_CONFIG_ERROR",
        SMB_NTSTATUS_FT_MISSING_MEMBER                                              => "STATUS_FT_MISSING_MEMBER",
        SMB_NTSTATUS_ILL_FORMED_SERVICE_ENTRY                                       => "STATUS_ILL_FORMED_SERVICE_ENTRY",
        SMB_NTSTATUS_ILLEGAL_CHARACTER                                              => "STATUS_ILLEGAL_CHARACTER",
        SMB_NTSTATUS_UNMAPPABLE_CHARACTER                                           => "STATUS_UNMAPPABLE_CHARACTER",
        SMB_NTSTATUS_UNDEFINED_CHARACTER                                            => "STATUS_UNDEFINED_CHARACTER",
        SMB_NTSTATUS_FLOPPY_VOLUME                                                  => "STATUS_FLOPPY_VOLUME",
        SMB_NTSTATUS_FLOPPY_ID_MARK_NOT_FOUND                                       => "STATUS_FLOPPY_ID_MARK_NOT_FOUND",
        SMB_NTSTATUS_FLOPPY_WRONG_CYLINDER                                          => "STATUS_FLOPPY_WRONG_CYLINDER",
        SMB_NTSTATUS_FLOPPY_UNKNOWN_ERROR                                           => "STATUS_FLOPPY_UNKNOWN_ERROR",
        SMB_NTSTATUS_FLOPPY_BAD_REGISTERS                                           => "STATUS_FLOPPY_BAD_REGISTERS",
        SMB_NTSTATUS_DISK_RECALIBRATE_FAILED                                        => "STATUS_DISK_RECALIBRATE_FAILED",
        SMB_NTSTATUS_DISK_OPERATION_FAILED                                          => "STATUS_DISK_OPERATION_FAILED",
        SMB_NTSTATUS_DISK_RESET_FAILED                                              => "STATUS_DISK_RESET_FAILED",
        SMB_NTSTATUS_SHARED_IRQ_BUSY                                                => "STATUS_SHARED_IRQ_BUSY",
        SMB_NTSTATUS_FT_ORPHANING                                                   => "STATUS_FT_ORPHANING",
        SMB_NTSTATUS_BIOS_FAILED_TO_CONNECT_INTERRUPT                               => "STATUS_BIOS_FAILED_TO_CONNECT_INTERRUPT",
        SMB_NTSTATUS_PARTITION_FAILURE                                              => "STATUS_PARTITION_FAILURE",
        SMB_NTSTATUS_INVALID_BLOCK_LENGTH                                           => "STATUS_INVALID_BLOCK_LENGTH",
        SMB_NTSTATUS_DEVICE_NOT_PARTITIONED                                         => "STATUS_DEVICE_NOT_PARTITIONED",
        SMB_NTSTATUS_UNABLE_TO_LOCK_MEDIA                                           => "STATUS_UNABLE_TO_LOCK_MEDIA",
        SMB_NTSTATUS_UNABLE_TO_UNLOAD_MEDIA                                         => "STATUS_UNABLE_TO_UNLOAD_MEDIA",
        SMB_NTSTATUS_EOM_OVERFLOW                                                   => "STATUS_EOM_OVERFLOW",
        SMB_NTSTATUS_NO_MEDIA                                                       => "STATUS_NO_MEDIA",
        SMB_NTSTATUS_NO_SUCH_MEMBER                                                 => "STATUS_NO_SUCH_MEMBER",
        SMB_NTSTATUS_INVALID_MEMBER                                                 => "STATUS_INVALID_MEMBER",
        SMB_NTSTATUS_KEY_DELETED                                                    => "STATUS_KEY_DELETED",
        SMB_NTSTATUS_NO_LOG_SPACE                                                   => "STATUS_NO_LOG_SPACE",
        SMB_NTSTATUS_TOO_MANY_SIDS                                                  => "STATUS_TOO_MANY_SIDS",
        SMB_NTSTATUS_LM_CROSS_ENCRYPTION_REQUIRED                                   => "STATUS_LM_CROSS_ENCRYPTION_REQUIRED",
        SMB_NTSTATUS_KEY_HAS_CHILDREN                                               => "STATUS_KEY_HAS_CHILDREN",
        SMB_NTSTATUS_CHILD_MUST_BE_VOLATILE                                         => "STATUS_CHILD_MUST_BE_VOLATILE",
        SMB_NTSTATUS_DEVICE_CONFIGURATION_ERROR                                     => "STATUS_DEVICE_CONFIGURATION_ERROR",
        SMB_NTSTATUS_DRIVER_INTERNAL_ERROR                                          => "STATUS_DRIVER_INTERNAL_ERROR",
        SMB_NTSTATUS_INVALID_DEVICE_STATE                                           => "STATUS_INVALID_DEVICE_STATE",
        SMB_NTSTATUS_IO_DEVICE_ERROR                                                => "STATUS_IO_DEVICE_ERROR",
        SMB_NTSTATUS_DEVICE_PROTOCOL_ERROR                                          => "STATUS_DEVICE_PROTOCOL_ERROR",
        SMB_NTSTATUS_BACKUP_CONTROLLER                                              => "STATUS_BACKUP_CONTROLLER",
        SMB_NTSTATUS_LOG_FILE_FULL                                                  => "STATUS_LOG_FILE_FULL",
        SMB_NTSTATUS_TOO_LATE                                                       => "STATUS_TOO_LATE",
        SMB_NTSTATUS_NO_TRUST_LSA_SECRET                                            => "STATUS_NO_TRUST_LSA_SECRET",
        SMB_NTSTATUS_NO_TRUST_SAM_ACCOUNT                                           => "STATUS_NO_TRUST_SAM_ACCOUNT",
        SMB_NTSTATUS_TRUSTED_DOMAIN_FAILURE                                         => "STATUS_TRUSTED_DOMAIN_FAILURE",
        SMB_NTSTATUS_TRUSTED_RELATIONSHIP_FAILURE                                   => "STATUS_TRUSTED_RELATIONSHIP_FAILURE",
        SMB_NTSTATUS_EVENTLOG_FILE_CORRUPT                                          => "STATUS_EVENTLOG_FILE_CORRUPT",
        SMB_NTSTATUS_EVENTLOG_CANT_START                                            => "STATUS_EVENTLOG_CANT_START",
        SMB_NTSTATUS_TRUST_FAILURE                                                  => "STATUS_TRUST_FAILURE",
        SMB_NTSTATUS_MUTANT_LIMIT_EXCEEDED                                          => "STATUS_MUTANT_LIMIT_EXCEEDED",
        SMB_NTSTATUS_NETLOGON_NOT_STARTED                                           => "STATUS_NETLOGON_NOT_STARTED",
        SMB_NTSTATUS_ACCOUNT_EXPIRED                                                => "STATUS_ACCOUNT_EXPIRED",
        SMB_NTSTATUS_POSSIBLE_DEADLOCK                                              => "STATUS_POSSIBLE_DEADLOCK",
        SMB_NTSTATUS_NETWORK_CREDENTIAL_CONFLICT                                    => "STATUS_NETWORK_CREDENTIAL_CONFLICT",
        SMB_NTSTATUS_REMOTE_SESSION_LIMIT                                           => "STATUS_REMOTE_SESSION_LIMIT",
        SMB_NTSTATUS_EVENTLOG_FILE_CHANGED                                          => "STATUS_EVENTLOG_FILE_CHANGED",
        SMB_NTSTATUS_NOLOGON_INTERDOMAIN_TRUST_ACCOUNT                              => "STATUS_NOLOGON_INTERDOMAIN_TRUST_ACCOUNT",
        SMB_NTSTATUS_NOLOGON_WORKSTATION_TRUST_ACCOUNT                              => "STATUS_NOLOGON_WORKSTATION_TRUST_ACCOUNT",
        SMB_NTSTATUS_NOLOGON_SERVER_TRUST_ACCOUNT                                   => "STATUS_NOLOGON_SERVER_TRUST_ACCOUNT",
        SMB_NTSTATUS_DOMAIN_TRUST_INCONSISTENT                                      => "STATUS_DOMAIN_TRUST_INCONSISTENT",
        SMB_NTSTATUS_FS_DRIVER_REQUIRED                                             => "STATUS_FS_DRIVER_REQUIRED",
        SMB_NTSTATUS_IMAGE_ALREADY_LOADED_AS_DLL                                    => "STATUS_IMAGE_ALREADY_LOADED_AS_DLL",
        SMB_NTSTATUS_INCOMPATIBLE_WITH_GLOBAL_SHORT_NAME_REGISTRY_SETTING           => "STATUS_INCOMPATIBLE_WITH_GLOBAL_SHORT_NAME_REGISTRY_SETTING",
        SMB_NTSTATUS_SHORT_NAMES_NOT_ENABLED_ON_VOLUME                              => "STATUS_SHORT_NAMES_NOT_ENABLED_ON_VOLUME",
        SMB_NTSTATUS_SECURITY_STREAM_IS_INCONSISTENT                                => "STATUS_SECURITY_STREAM_IS_INCONSISTENT",
        SMB_NTSTATUS_INVALID_LOCK_RANGE                                             => "STATUS_INVALID_LOCK_RANGE",
        SMB_NTSTATUS_INVALID_ACE_CONDITION                                          => "STATUS_INVALID_ACE_CONDITION",
        SMB_NTSTATUS_IMAGE_SUBSYSTEM_NOT_PRESENT                                    => "STATUS_IMAGE_SUBSYSTEM_NOT_PRESENT",
        SMB_NTSTATUS_NOTIFICATION_GUID_ALREADY_DEFINED                              => "STATUS_NOTIFICATION_GUID_ALREADY_DEFINED",
        SMB_NTSTATUS_NETWORK_OPEN_RESTRICTION                                       => "STATUS_NETWORK_OPEN_RESTRICTION",
        SMB_NTSTATUS_NO_USER_SESSION_KEY                                            => "STATUS_NO_USER_SESSION_KEY",
        SMB_NTSTATUS_USER_SESSION_DELETED                                           => "STATUS_USER_SESSION_DELETED",
        SMB_NTSTATUS_RESOURCE_LANG_NOT_FOUND                                        => "STATUS_RESOURCE_LANG_NOT_FOUND",
        SMB_NTSTATUS_INSUFF_SERVER_RESOURCES                                        => "STATUS_INSUFF_SERVER_RESOURCES",
        SMB_NTSTATUS_INVALID_BUFFER_SIZE                                            => "STATUS_INVALID_BUFFER_SIZE",
        SMB_NTSTATUS_INVALID_ADDRESS_COMPONENT                                      => "STATUS_INVALID_ADDRESS_COMPONENT",
        SMB_NTSTATUS_INVALID_ADDRESS_WILDCARD                                       => "STATUS_INVALID_ADDRESS_WILDCARD",
        SMB_NTSTATUS_TOO_MANY_ADDRESSES                                             => "STATUS_TOO_MANY_ADDRESSES",
        SMB_NTSTATUS_ADDRESS_ALREADY_EXISTS                                         => "STATUS_ADDRESS_ALREADY_EXISTS",
        SMB_NTSTATUS_ADDRESS_CLOSED                                                 => "STATUS_ADDRESS_CLOSED",
        SMB_NTSTATUS_CONNECTION_DISCONNECTED                                        => "STATUS_CONNECTION_DISCONNECTED",
        SMB_NTSTATUS_CONNECTION_RESET                                               => "STATUS_CONNECTION_RESET",
        SMB_NTSTATUS_TOO_MANY_NODES                                                 => "STATUS_TOO_MANY_NODES",
        SMB_NTSTATUS_TRANSACTION_ABORTED                                            => "STATUS_TRANSACTION_ABORTED",
        SMB_NTSTATUS_TRANSACTION_TIMED_OUT                                          => "STATUS_TRANSACTION_TIMED_OUT",
        SMB_NTSTATUS_TRANSACTION_NO_RELEASE                                         => "STATUS_TRANSACTION_NO_RELEASE",
        SMB_NTSTATUS_TRANSACTION_NO_MATCH                                           => "STATUS_TRANSACTION_NO_MATCH",
        SMB_NTSTATUS_TRANSACTION_RESPONDED                                          => "STATUS_TRANSACTION_RESPONDED",
        SMB_NTSTATUS_TRANSACTION_INVALID_ID                                         => "STATUS_TRANSACTION_INVALID_ID",
        SMB_NTSTATUS_TRANSACTION_INVALID_TYPE                                       => "STATUS_TRANSACTION_INVALID_TYPE",
        SMB_NTSTATUS_NOT_SERVER_SESSION                                             => "STATUS_NOT_SERVER_SESSION",
        SMB_NTSTATUS_NOT_CLIENT_SESSION                                             => "STATUS_NOT_CLIENT_SESSION",
        SMB_NTSTATUS_CANNOT_LOAD_REGISTRY_FILE                                      => "STATUS_CANNOT_LOAD_REGISTRY_FILE",
        SMB_NTSTATUS_DEBUG_ATTACH_FAILED                                            => "STATUS_DEBUG_ATTACH_FAILED",
        SMB_NTSTATUS_SYSTEM_PROCESS_TERMINATED                                      => "STATUS_SYSTEM_PROCESS_TERMINATED",
        SMB_NTSTATUS_DATA_NOT_ACCEPTED                                              => "STATUS_DATA_NOT_ACCEPTED",
        SMB_NTSTATUS_NO_BROWSER_SERVERS_FOUND                                       => "STATUS_NO_BROWSER_SERVERS_FOUND",
        SMB_NTSTATUS_VDM_HARD_ERROR                                                 => "STATUS_VDM_HARD_ERROR",
        SMB_NTSTATUS_DRIVER_CANCEL_TIMEOUT                                          => "STATUS_DRIVER_CANCEL_TIMEOUT",
        SMB_NTSTATUS_REPLY_MESSAGE_MISMATCH                                         => "STATUS_REPLY_MESSAGE_MISMATCH",
        SMB_NTSTATUS_MAPPED_ALIGNMENT                                               => "STATUS_MAPPED_ALIGNMENT",
        SMB_NTSTATUS_IMAGE_CHECKSUM_MISMATCH                                        => "STATUS_IMAGE_CHECKSUM_MISMATCH",
        SMB_NTSTATUS_LOST_WRITEBEHIND_DATA                                          => "STATUS_LOST_WRITEBEHIND_DATA",
        SMB_NTSTATUS_CLIENT_SERVER_PARAMETERS_INVALID                               => "STATUS_CLIENT_SERVER_PARAMETERS_INVALID",
        SMB_NTSTATUS_PASSWORD_MUST_CHANGE                                           => "STATUS_PASSWORD_MUST_CHANGE",
        SMB_NTSTATUS_NOT_FOUND                                                      => "STATUS_NOT_FOUND",
        SMB_NTSTATUS_NOT_TINY_STREAM                                                => "STATUS_NOT_TINY_STREAM",
        SMB_NTSTATUS_RECOVERY_FAILURE                                               => "STATUS_RECOVERY_FAILURE",
        SMB_NTSTATUS_STACK_OVERFLOW_READ                                            => "STATUS_STACK_OVERFLOW_READ",
        SMB_NTSTATUS_FAIL_CHECK                                                     => "STATUS_FAIL_CHECK",
        SMB_NTSTATUS_DUPLICATE_OBJECTID                                             => "STATUS_DUPLICATE_OBJECTID",
        SMB_NTSTATUS_OBJECTID_EXISTS                                                => "STATUS_OBJECTID_EXISTS",
        SMB_NTSTATUS_CONVERT_TO_LARGE                                               => "STATUS_CONVERT_TO_LARGE",
        SMB_NTSTATUS_RETRY                                                          => "STATUS_RETRY",
        SMB_NTSTATUS_FOUND_OUT_OF_SCOPE                                             => "STATUS_FOUND_OUT_OF_SCOPE",
        SMB_NTSTATUS_ALLOCATE_BUCKET                                                => "STATUS_ALLOCATE_BUCKET",
        SMB_NTSTATUS_PROPSET_NOT_FOUND                                              => "STATUS_PROPSET_NOT_FOUND",
        SMB_NTSTATUS_MARSHALL_OVERFLOW                                              => "STATUS_MARSHALL_OVERFLOW",
        SMB_NTSTATUS_INVALID_VARIANT                                                => "STATUS_INVALID_VARIANT",
        SMB_NTSTATUS_DOMAIN_CONTROLLER_NOT_FOUND                                    => "STATUS_DOMAIN_CONTROLLER_NOT_FOUND",
        SMB_NTSTATUS_ACCOUNT_LOCKED_OUT                                             => "STATUS_ACCOUNT_LOCKED_OUT",
        SMB_NTSTATUS_HANDLE_NOT_CLOSABLE                                            => "STATUS_HANDLE_NOT_CLOSABLE",
        SMB_NTSTATUS_CONNECTION_REFUSED                                             => "STATUS_CONNECTION_REFUSED",
        SMB_NTSTATUS_GRACEFUL_DISCONNECT                                            => "STATUS_GRACEFUL_DISCONNECT",
        SMB_NTSTATUS_ADDRESS_ALREADY_ASSOCIATED                                     => "STATUS_ADDRESS_ALREADY_ASSOCIATED",
        SMB_NTSTATUS_ADDRESS_NOT_ASSOCIATED                                         => "STATUS_ADDRESS_NOT_ASSOCIATED",
        SMB_NTSTATUS_CONNECTION_INVALID                                             => "STATUS_CONNECTION_INVALID",
        SMB_NTSTATUS_CONNECTION_ACTIVE                                              => "STATUS_CONNECTION_ACTIVE",
        SMB_NTSTATUS_NETWORK_UNREACHABLE                                            => "STATUS_NETWORK_UNREACHABLE",
        SMB_NTSTATUS_HOST_UNREACHABLE                                               => "STATUS_HOST_UNREACHABLE",
        SMB_NTSTATUS_PROTOCOL_UNREACHABLE                                           => "STATUS_PROTOCOL_UNREACHABLE",
        SMB_NTSTATUS_PORT_UNREACHABLE                                               => "STATUS_PORT_UNREACHABLE",
        SMB_NTSTATUS_REQUEST_ABORTED                                                => "STATUS_REQUEST_ABORTED",
        SMB_NTSTATUS_CONNECTION_ABORTED                                             => "STATUS_CONNECTION_ABORTED",
        SMB_NTSTATUS_BAD_COMPRESSION_BUFFER                                         => "STATUS_BAD_COMPRESSION_BUFFER",
        SMB_NTSTATUS_USER_MAPPED_FILE                                               => "STATUS_USER_MAPPED_FILE",
        SMB_NTSTATUS_AUDIT_FAILED                                                   => "STATUS_AUDIT_FAILED",
        SMB_NTSTATUS_TIMER_RESOLUTION_NOT_SET                                       => "STATUS_TIMER_RESOLUTION_NOT_SET",
        SMB_NTSTATUS_CONNECTION_COUNT_LIMIT                                         => "STATUS_CONNECTION_COUNT_LIMIT",
        SMB_NTSTATUS_LOGIN_TIME_RESTRICTION                                         => "STATUS_LOGIN_TIME_RESTRICTION",
        SMB_NTSTATUS_LOGIN_WKSTA_RESTRICTION                                        => "STATUS_LOGIN_WKSTA_RESTRICTION",
        SMB_NTSTATUS_IMAGE_MP_UP_MISMATCH                                           => "STATUS_IMAGE_MP_UP_MISMATCH",
        SMB_NTSTATUS_INSUFFICIENT_LOGON_INFO                                        => "STATUS_INSUFFICIENT_LOGON_INFO",
        SMB_NTSTATUS_BAD_DLL_ENTRYPOINT                                             => "STATUS_BAD_DLL_ENTRYPOINT",
        SMB_NTSTATUS_BAD_SERVICE_ENTRYPOINT                                         => "STATUS_BAD_SERVICE_ENTRYPOINT",
        SMB_NTSTATUS_LPC_REPLY_LOST                                                 => "STATUS_LPC_REPLY_LOST",
        SMB_NTSTATUS_IP_ADDRESS_CONFLICT1                                           => "STATUS_IP_ADDRESS_CONFLICT1",
        SMB_NTSTATUS_IP_ADDRESS_CONFLICT2                                           => "STATUS_IP_ADDRESS_CONFLICT2",
        SMB_NTSTATUS_REGISTRY_QUOTA_LIMIT                                           => "STATUS_REGISTRY_QUOTA_LIMIT",
        SMB_NTSTATUS_PATH_NOT_COVERED                                               => "STATUS_PATH_NOT_COVERED",
        SMB_NTSTATUS_NO_CALLBACK_ACTIVE                                             => "STATUS_NO_CALLBACK_ACTIVE",
        SMB_NTSTATUS_LICENSE_QUOTA_EXCEEDED                                         => "STATUS_LICENSE_QUOTA_EXCEEDED",
        SMB_NTSTATUS_PWD_TOO_SHORT                                                  => "STATUS_PWD_TOO_SHORT",
        SMB_NTSTATUS_PWD_TOO_RECENT                                                 => "STATUS_PWD_TOO_RECENT",
        SMB_NTSTATUS_PWD_HISTORY_CONFLICT                                           => "STATUS_PWD_HISTORY_CONFLICT",
        SMB_NTSTATUS_PLUGPLAY_NO_DEVICE                                             => "STATUS_PLUGPLAY_NO_DEVICE",
        SMB_NTSTATUS_UNSUPPORTED_COMPRESSION                                        => "STATUS_UNSUPPORTED_COMPRESSION",
        SMB_NTSTATUS_INVALID_HW_PROFILE                                             => "STATUS_INVALID_HW_PROFILE",
        SMB_NTSTATUS_INVALID_PLUGPLAY_DEVICE_PATH                                   => "STATUS_INVALID_PLUGPLAY_DEVICE_PATH",
        SMB_NTSTATUS_DRIVER_ORDINAL_NOT_FOUND                                       => "STATUS_DRIVER_ORDINAL_NOT_FOUND",
        SMB_NTSTATUS_DRIVER_ENTRYPOINT_NOT_FOUND                                    => "STATUS_DRIVER_ENTRYPOINT_NOT_FOUND",
        SMB_NTSTATUS_RESOURCE_NOT_OWNED                                             => "STATUS_RESOURCE_NOT_OWNED",
        SMB_NTSTATUS_TOO_MANY_LINKS                                                 => "STATUS_TOO_MANY_LINKS",
        SMB_NTSTATUS_QUOTA_LIST_INCONSISTENT                                        => "STATUS_QUOTA_LIST_INCONSISTENT",
        SMB_NTSTATUS_FILE_IS_OFFLINE                                                => "STATUS_FILE_IS_OFFLINE",
        SMB_NTSTATUS_EVALUATION_EXPIRATION                                          => "STATUS_EVALUATION_EXPIRATION",
        SMB_NTSTATUS_ILLEGAL_DLL_RELOCATION                                         => "STATUS_ILLEGAL_DLL_RELOCATION",
        SMB_NTSTATUS_LICENSE_VIOLATION                                              => "STATUS_LICENSE_VIOLATION",
        SMB_NTSTATUS_DLL_INIT_FAILED_LOGOFF                                         => "STATUS_DLL_INIT_FAILED_LOGOFF",
        SMB_NTSTATUS_DRIVER_UNABLE_TO_LOAD                                          => "STATUS_DRIVER_UNABLE_TO_LOAD",
        SMB_NTSTATUS_DFS_UNAVAILABLE                                                => "STATUS_DFS_UNAVAILABLE",
        SMB_NTSTATUS_VOLUME_DISMOUNTED                                              => "STATUS_VOLUME_DISMOUNTED",
        SMB_NTSTATUS_WX86_INTERNAL_ERROR                                            => "STATUS_WX86_INTERNAL_ERROR",
        SMB_NTSTATUS_WX86_FLOAT_STACK_CHECK                                         => "STATUS_WX86_FLOAT_STACK_CHECK",
        SMB_NTSTATUS_VALIDATE_CONTINUE                                              => "STATUS_VALIDATE_CONTINUE",
        SMB_NTSTATUS_NO_MATCH                                                       => "STATUS_NO_MATCH",
        SMB_NTSTATUS_NO_MORE_MATCHES                                                => "STATUS_NO_MORE_MATCHES",
        SMB_NTSTATUS_NOT_A_REPARSE_POINT                                            => "STATUS_NOT_A_REPARSE_POINT",
        SMB_NTSTATUS_IO_REPARSE_TAG_INVALID                                         => "STATUS_IO_REPARSE_TAG_INVALID",
        SMB_NTSTATUS_IO_REPARSE_TAG_MISMATCH                                        => "STATUS_IO_REPARSE_TAG_MISMATCH",
        SMB_NTSTATUS_IO_REPARSE_DATA_INVALID                                        => "STATUS_IO_REPARSE_DATA_INVALID",
        SMB_NTSTATUS_IO_REPARSE_TAG_NOT_HANDLED                                     => "STATUS_IO_REPARSE_TAG_NOT_HANDLED",
        SMB_NTSTATUS_REPARSE_POINT_NOT_RESOLVED                                     => "STATUS_REPARSE_POINT_NOT_RESOLVED",
        SMB_NTSTATUS_DIRECTORY_IS_A_REPARSE_POINT                                   => "STATUS_DIRECTORY_IS_A_REPARSE_POINT",
        SMB_NTSTATUS_RANGE_LIST_CONFLICT                                            => "STATUS_RANGE_LIST_CONFLICT",
        SMB_NTSTATUS_SOURCE_ELEMENT_EMPTY                                           => "STATUS_SOURCE_ELEMENT_EMPTY",
        SMB_NTSTATUS_DESTINATION_ELEMENT_FULL                                       => "STATUS_DESTINATION_ELEMENT_FULL",
        SMB_NTSTATUS_ILLEGAL_ELEMENT_ADDRESS                                        => "STATUS_ILLEGAL_ELEMENT_ADDRESS",
        SMB_NTSTATUS_MAGAZINE_NOT_PRESENT                                           => "STATUS_MAGAZINE_NOT_PRESENT",
        SMB_NTSTATUS_REINITIALIZATION_NEEDED                                        => "STATUS_REINITIALIZATION_NEEDED",
        SMB_NTSTATUS_ENCRYPTION_FAILED                                              => "STATUS_ENCRYPTION_FAILED",
        SMB_NTSTATUS_DECRYPTION_FAILED                                              => "STATUS_DECRYPTION_FAILED",
        SMB_NTSTATUS_RANGE_NOT_FOUND                                                => "STATUS_RANGE_NOT_FOUND",
        SMB_NTSTATUS_NO_RECOVERY_POLICY                                             => "STATUS_NO_RECOVERY_POLICY",
        SMB_NTSTATUS_NO_EFS                                                         => "STATUS_NO_EFS",
        SMB_NTSTATUS_WRONG_EFS                                                      => "STATUS_WRONG_EFS",
        SMB_NTSTATUS_NO_USER_KEYS                                                   => "STATUS_NO_USER_KEYS",
        SMB_NTSTATUS_FILE_NOT_ENCRYPTED                                             => "STATUS_FILE_NOT_ENCRYPTED",
        SMB_NTSTATUS_NOT_EXPORT_FORMAT                                              => "STATUS_NOT_EXPORT_FORMAT",
        SMB_NTSTATUS_FILE_ENCRYPTED                                                 => "STATUS_FILE_ENCRYPTED",
        SMB_NTSTATUS_WMI_GUID_NOT_FOUND                                             => "STATUS_WMI_GUID_NOT_FOUND",
        SMB_NTSTATUS_WMI_INSTANCE_NOT_FOUND                                         => "STATUS_WMI_INSTANCE_NOT_FOUND",
        SMB_NTSTATUS_WMI_ITEMID_NOT_FOUND                                           => "STATUS_WMI_ITEMID_NOT_FOUND",
        SMB_NTSTATUS_WMI_TRY_AGAIN                                                  => "STATUS_WMI_TRY_AGAIN",
        SMB_NTSTATUS_SHARED_POLICY                                                  => "STATUS_SHARED_POLICY",
        SMB_NTSTATUS_POLICY_OBJECT_NOT_FOUND                                        => "STATUS_POLICY_OBJECT_NOT_FOUND",
        SMB_NTSTATUS_POLICY_ONLY_IN_DS                                              => "STATUS_POLICY_ONLY_IN_DS",
        SMB_NTSTATUS_VOLUME_NOT_UPGRADED                                            => "STATUS_VOLUME_NOT_UPGRADED",
        SMB_NTSTATUS_REMOTE_STORAGE_NOT_ACTIVE                                      => "STATUS_REMOTE_STORAGE_NOT_ACTIVE",
        SMB_NTSTATUS_REMOTE_STORAGE_MEDIA_ERROR                                     => "STATUS_REMOTE_STORAGE_MEDIA_ERROR",
        SMB_NTSTATUS_NO_TRACKING_SERVICE                                            => "STATUS_NO_TRACKING_SERVICE",
        SMB_NTSTATUS_SERVER_SID_MISMATCH                                            => "STATUS_SERVER_SID_MISMATCH",
        SMB_NTSTATUS_DS_NO_ATTRIBUTE_OR_VALUE                                       => "STATUS_DS_NO_ATTRIBUTE_OR_VALUE",
        SMB_NTSTATUS_DS_INVALID_ATTRIBUTE_SYNTAX                                    => "STATUS_DS_INVALID_ATTRIBUTE_SYNTAX",
        SMB_NTSTATUS_DS_ATTRIBUTE_TYPE_UNDEFINED                                    => "STATUS_DS_ATTRIBUTE_TYPE_UNDEFINED",
        SMB_NTSTATUS_DS_ATTRIBUTE_OR_VALUE_EXISTS                                   => "STATUS_DS_ATTRIBUTE_OR_VALUE_EXISTS",
        SMB_NTSTATUS_DS_BUSY                                                        => "STATUS_DS_BUSY",
        SMB_NTSTATUS_DS_UNAVAILABLE                                                 => "STATUS_DS_UNAVAILABLE",
        SMB_NTSTATUS_DS_NO_RIDS_ALLOCATED                                           => "STATUS_DS_NO_RIDS_ALLOCATED",
        SMB_NTSTATUS_DS_NO_MORE_RIDS                                                => "STATUS_DS_NO_MORE_RIDS",
        SMB_NTSTATUS_DS_INCORRECT_ROLE_OWNER                                        => "STATUS_DS_INCORRECT_ROLE_OWNER",
        SMB_NTSTATUS_DS_RIDMGR_INIT_ERROR                                           => "STATUS_DS_RIDMGR_INIT_ERROR",
        SMB_NTSTATUS_DS_OBJ_CLASS_VIOLATION                                         => "STATUS_DS_OBJ_CLASS_VIOLATION",
        SMB_NTSTATUS_DS_CANT_ON_NON_LEAF                                            => "STATUS_DS_CANT_ON_NON_LEAF",
        SMB_NTSTATUS_DS_CANT_ON_RDN                                                 => "STATUS_DS_CANT_ON_RDN",
        SMB_NTSTATUS_DS_CANT_MOD_OBJ_CLASS                                          => "STATUS_DS_CANT_MOD_OBJ_CLASS",
        SMB_NTSTATUS_DS_CROSS_DOM_MOVE_FAILED                                       => "STATUS_DS_CROSS_DOM_MOVE_FAILED",
        SMB_NTSTATUS_DS_GC_NOT_AVAILABLE                                            => "STATUS_DS_GC_NOT_AVAILABLE",
        SMB_NTSTATUS_DIRECTORY_SERVICE_REQUIRED                                     => "STATUS_DIRECTORY_SERVICE_REQUIRED",
        SMB_NTSTATUS_REPARSE_ATTRIBUTE_CONFLICT                                     => "STATUS_REPARSE_ATTRIBUTE_CONFLICT",
        SMB_NTSTATUS_CANT_ENABLE_DENY_ONLY                                          => "STATUS_CANT_ENABLE_DENY_ONLY",
        SMB_NTSTATUS_FLOAT_MULTIPLE_FAULTS                                          => "STATUS_FLOAT_MULTIPLE_FAULTS",
        SMB_NTSTATUS_FLOAT_MULTIPLE_TRAPS                                           => "STATUS_FLOAT_MULTIPLE_TRAPS",
        SMB_NTSTATUS_DEVICE_REMOVED                                                 => "STATUS_DEVICE_REMOVED",
        SMB_NTSTATUS_JOURNAL_DELETE_IN_PROGRESS                                     => "STATUS_JOURNAL_DELETE_IN_PROGRESS",
        SMB_NTSTATUS_JOURNAL_NOT_ACTIVE                                             => "STATUS_JOURNAL_NOT_ACTIVE",
        SMB_NTSTATUS_NOINTERFACE                                                    => "STATUS_NOINTERFACE",
        SMB_NTSTATUS_DS_ADMIN_LIMIT_EXCEEDED                                        => "STATUS_DS_ADMIN_LIMIT_EXCEEDED",
        SMB_NTSTATUS_DRIVER_FAILED_SLEEP                                            => "STATUS_DRIVER_FAILED_SLEEP",
        SMB_NTSTATUS_MUTUAL_AUTHENTICATION_FAILED                                   => "STATUS_MUTUAL_AUTHENTICATION_FAILED",
        SMB_NTSTATUS_CORRUPT_SYSTEM_FILE                                            => "STATUS_CORRUPT_SYSTEM_FILE",
        SMB_NTSTATUS_DATATYPE_MISALIGNMENT_ERROR                                    => "STATUS_DATATYPE_MISALIGNMENT_ERROR",
        SMB_NTSTATUS_WMI_READ_ONLY                                                  => "STATUS_WMI_READ_ONLY",
        SMB_NTSTATUS_WMI_SET_FAILURE                                                => "STATUS_WMI_SET_FAILURE",
        SMB_NTSTATUS_COMMITMENT_MINIMUM                                             => "STATUS_COMMITMENT_MINIMUM",
        SMB_NTSTATUS_REG_NAT_CONSUMPTION                                            => "STATUS_REG_NAT_CONSUMPTION",
        SMB_NTSTATUS_TRANSPORT_FULL                                                 => "STATUS_TRANSPORT_FULL",
        SMB_NTSTATUS_DS_SAM_INIT_FAILURE                                            => "STATUS_DS_SAM_INIT_FAILURE",
        SMB_NTSTATUS_ONLY_IF_CONNECTED                                              => "STATUS_ONLY_IF_CONNECTED",
        SMB_NTSTATUS_DS_SENSITIVE_GROUP_VIOLATION                                   => "STATUS_DS_SENSITIVE_GROUP_VIOLATION",
        SMB_NTSTATUS_PNP_RESTART_ENUMERATION                                        => "STATUS_PNP_RESTART_ENUMERATION",
        SMB_NTSTATUS_JOURNAL_ENTRY_DELETED                                          => "STATUS_JOURNAL_ENTRY_DELETED",
        SMB_NTSTATUS_DS_CANT_MOD_PRIMARYGROUPID                                     => "STATUS_DS_CANT_MOD_PRIMARYGROUPID",
        SMB_NTSTATUS_SYSTEM_IMAGE_BAD_SIGNATURE                                     => "STATUS_SYSTEM_IMAGE_BAD_SIGNATURE",
        SMB_NTSTATUS_PNP_REBOOT_REQUIRED                                            => "STATUS_PNP_REBOOT_REQUIRED",
        SMB_NTSTATUS_POWER_STATE_INVALID                                            => "STATUS_POWER_STATE_INVALID",
        SMB_NTSTATUS_DS_INVALID_GROUP_TYPE                                          => "STATUS_DS_INVALID_GROUP_TYPE",
        SMB_NTSTATUS_DS_NO_NEST_GLOBALGROUP_IN_MIXEDDOMAIN                          => "STATUS_DS_NO_NEST_GLOBALGROUP_IN_MIXEDDOMAIN",
        SMB_NTSTATUS_DS_NO_NEST_LOCALGROUP_IN_MIXEDDOMAIN                           => "STATUS_DS_NO_NEST_LOCALGROUP_IN_MIXEDDOMAIN",
        SMB_NTSTATUS_DS_GLOBAL_CANT_HAVE_LOCAL_MEMBER                               => "STATUS_DS_GLOBAL_CANT_HAVE_LOCAL_MEMBER",
        SMB_NTSTATUS_DS_GLOBAL_CANT_HAVE_UNIVERSAL_MEMBER                           => "STATUS_DS_GLOBAL_CANT_HAVE_UNIVERSAL_MEMBER",
        SMB_NTSTATUS_DS_UNIVERSAL_CANT_HAVE_LOCAL_MEMBER                            => "STATUS_DS_UNIVERSAL_CANT_HAVE_LOCAL_MEMBER",
        SMB_NTSTATUS_DS_GLOBAL_CANT_HAVE_CROSSDOMAIN_MEMBER                         => "STATUS_DS_GLOBAL_CANT_HAVE_CROSSDOMAIN_MEMBER",
        SMB_NTSTATUS_DS_LOCAL_CANT_HAVE_CROSSDOMAIN_LOCAL_MEMBER                    => "STATUS_DS_LOCAL_CANT_HAVE_CROSSDOMAIN_LOCAL_MEMBER",
        SMB_NTSTATUS_DS_HAVE_PRIMARY_MEMBERS                                        => "STATUS_DS_HAVE_PRIMARY_MEMBERS",
        SMB_NTSTATUS_WMI_NOT_SUPPORTED                                              => "STATUS_WMI_NOT_SUPPORTED",
        SMB_NTSTATUS_INSUFFICIENT_POWER                                             => "STATUS_INSUFFICIENT_POWER",
        SMB_NTSTATUS_SAM_NEED_BOOTKEY_PASSWORD                                      => "STATUS_SAM_NEED_BOOTKEY_PASSWORD",
        SMB_NTSTATUS_SAM_NEED_BOOTKEY_FLOPPY                                        => "STATUS_SAM_NEED_BOOTKEY_FLOPPY",
        SMB_NTSTATUS_DS_CANT_START                                                  => "STATUS_DS_CANT_START",
        SMB_NTSTATUS_DS_INIT_FAILURE                                                => "STATUS_DS_INIT_FAILURE",
        SMB_NTSTATUS_SAM_INIT_FAILURE                                               => "STATUS_SAM_INIT_FAILURE",
        SMB_NTSTATUS_DS_GC_REQUIRED                                                 => "STATUS_DS_GC_REQUIRED",
        SMB_NTSTATUS_DS_LOCAL_MEMBER_OF_LOCAL_ONLY                                  => "STATUS_DS_LOCAL_MEMBER_OF_LOCAL_ONLY",
        SMB_NTSTATUS_DS_NO_FPO_IN_UNIVERSAL_GROUPS                                  => "STATUS_DS_NO_FPO_IN_UNIVERSAL_GROUPS",
        SMB_NTSTATUS_DS_MACHINE_ACCOUNT_QUOTA_EXCEEDED                              => "STATUS_DS_MACHINE_ACCOUNT_QUOTA_EXCEEDED",
        SMB_NTSTATUS_CURRENT_DOMAIN_NOT_ALLOWED                                     => "STATUS_CURRENT_DOMAIN_NOT_ALLOWED",
        SMB_NTSTATUS_CANNOT_MAKE                                                    => "STATUS_CANNOT_MAKE",
        SMB_NTSTATUS_SYSTEM_SHUTDOWN                                                => "STATUS_SYSTEM_SHUTDOWN",
        SMB_NTSTATUS_DS_INIT_FAILURE_CONSOLE                                        => "STATUS_DS_INIT_FAILURE_CONSOLE",
        SMB_NTSTATUS_DS_SAM_INIT_FAILURE_CONSOLE                                    => "STATUS_DS_SAM_INIT_FAILURE_CONSOLE",
        SMB_NTSTATUS_UNFINISHED_CONTEXT_DELETED                                     => "STATUS_UNFINISHED_CONTEXT_DELETED",
        SMB_NTSTATUS_NO_TGT_REPLY                                                   => "STATUS_NO_TGT_REPLY",
        SMB_NTSTATUS_OBJECTID_NOT_FOUND                                             => "STATUS_OBJECTID_NOT_FOUND",
        SMB_NTSTATUS_NO_IP_ADDRESSES                                                => "STATUS_NO_IP_ADDRESSES",
        SMB_NTSTATUS_WRONG_CREDENTIAL_HANDLE                                        => "STATUS_WRONG_CREDENTIAL_HANDLE",
        SMB_NTSTATUS_CRYPTO_SYSTEM_INVALID                                          => "STATUS_CRYPTO_SYSTEM_INVALID",
        SMB_NTSTATUS_MAX_REFERRALS_EXCEEDED                                         => "STATUS_MAX_REFERRALS_EXCEEDED",
        SMB_NTSTATUS_MUST_BE_KDC                                                    => "STATUS_MUST_BE_KDC",
        SMB_NTSTATUS_STRONG_CRYPTO_NOT_SUPPORTED                                    => "STATUS_STRONG_CRYPTO_NOT_SUPPORTED",
        SMB_NTSTATUS_TOO_MANY_PRINCIPALS                                            => "STATUS_TOO_MANY_PRINCIPALS",
        SMB_NTSTATUS_NO_PA_DATA                                                     => "STATUS_NO_PA_DATA",
        SMB_NTSTATUS_PKINIT_NAME_MISMATCH                                           => "STATUS_PKINIT_NAME_MISMATCH",
        SMB_NTSTATUS_SMARTCARD_LOGON_REQUIRED                                       => "STATUS_SMARTCARD_LOGON_REQUIRED",
        SMB_NTSTATUS_KDC_INVALID_REQUEST                                            => "STATUS_KDC_INVALID_REQUEST",
        SMB_NTSTATUS_KDC_UNABLE_TO_REFER                                            => "STATUS_KDC_UNABLE_TO_REFER",
        SMB_NTSTATUS_KDC_UNKNOWN_ETYPE                                              => "STATUS_KDC_UNKNOWN_ETYPE",
        SMB_NTSTATUS_SHUTDOWN_IN_PROGRESS                                           => "STATUS_SHUTDOWN_IN_PROGRESS",
        SMB_NTSTATUS_SERVER_SHUTDOWN_IN_PROGRESS                                    => "STATUS_SERVER_SHUTDOWN_IN_PROGRESS",
        SMB_NTSTATUS_NOT_SUPPORTED_ON_SBS                                           => "STATUS_NOT_SUPPORTED_ON_SBS",
        SMB_NTSTATUS_WMI_GUID_DISCONNECTED                                          => "STATUS_WMI_GUID_DISCONNECTED",
        SMB_NTSTATUS_WMI_ALREADY_DISABLED                                           => "STATUS_WMI_ALREADY_DISABLED",
        SMB_NTSTATUS_WMI_ALREADY_ENABLED                                            => "STATUS_WMI_ALREADY_ENABLED",
        SMB_NTSTATUS_MFT_TOO_FRAGMENTED                                             => "STATUS_MFT_TOO_FRAGMENTED",
        SMB_NTSTATUS_COPY_PROTECTION_FAILURE                                        => "STATUS_COPY_PROTECTION_FAILURE",
        SMB_NTSTATUS_CSS_AUTHENTICATION_FAILURE                                     => "STATUS_CSS_AUTHENTICATION_FAILURE",
        SMB_NTSTATUS_CSS_KEY_NOT_PRESENT                                            => "STATUS_CSS_KEY_NOT_PRESENT",
        SMB_NTSTATUS_CSS_KEY_NOT_ESTABLISHED                                        => "STATUS_CSS_KEY_NOT_ESTABLISHED",
        SMB_NTSTATUS_CSS_SCRAMBLED_SECTOR                                           => "STATUS_CSS_SCRAMBLED_SECTOR",
        SMB_NTSTATUS_CSS_REGION_MISMATCH                                            => "STATUS_CSS_REGION_MISMATCH",
        SMB_NTSTATUS_CSS_RESETS_EXHAUSTED                                           => "STATUS_CSS_RESETS_EXHAUSTED",
        SMB_NTSTATUS_PKINIT_FAILURE                                                 => "STATUS_PKINIT_FAILURE",
        SMB_NTSTATUS_SMARTCARD_SUBSYSTEM_FAILURE                                    => "STATUS_SMARTCARD_SUBSYSTEM_FAILURE",
        SMB_NTSTATUS_NO_KERB_KEY                                                    => "STATUS_NO_KERB_KEY",
        SMB_NTSTATUS_HOST_DOWN                                                      => "STATUS_HOST_DOWN",
        SMB_NTSTATUS_UNSUPPORTED_PREAUTH                                            => "STATUS_UNSUPPORTED_PREAUTH",
        SMB_NTSTATUS_EFS_ALG_BLOB_TOO_BIG                                           => "STATUS_EFS_ALG_BLOB_TOO_BIG",
        SMB_NTSTATUS_PORT_NOT_SET                                                   => "STATUS_PORT_NOT_SET",
        SMB_NTSTATUS_DEBUGGER_INACTIVE                                              => "STATUS_DEBUGGER_INACTIVE",
        SMB_NTSTATUS_DS_VERSION_CHECK_FAILURE                                       => "STATUS_DS_VERSION_CHECK_FAILURE",
        SMB_NTSTATUS_AUDITING_DISABLED                                              => "STATUS_AUDITING_DISABLED",
        SMB_NTSTATUS_PRENT4_MACHINE_ACCOUNT                                         => "STATUS_PRENT4_MACHINE_ACCOUNT",
        SMB_NTSTATUS_DS_AG_CANT_HAVE_UNIVERSAL_MEMBER                               => "STATUS_DS_AG_CANT_HAVE_UNIVERSAL_MEMBER",
        SMB_NTSTATUS_INVALID_IMAGE_WIN_32                                           => "STATUS_INVALID_IMAGE_WIN_32",
        SMB_NTSTATUS_INVALID_IMAGE_WIN_64                                           => "STATUS_INVALID_IMAGE_WIN_64",
        SMB_NTSTATUS_BAD_BINDINGS                                                   => "STATUS_BAD_BINDINGS",
        SMB_NTSTATUS_NETWORK_SESSION_EXPIRED                                        => "STATUS_NETWORK_SESSION_EXPIRED",
        SMB_NTSTATUS_APPHELP_BLOCK                                                  => "STATUS_APPHELP_BLOCK",
        SMB_NTSTATUS_ALL_SIDS_FILTERED                                              => "STATUS_ALL_SIDS_FILTERED",
        SMB_NTSTATUS_NOT_SAFE_MODE_DRIVER                                           => "STATUS_NOT_SAFE_MODE_DRIVER",
        SMB_NTSTATUS_ACCESS_DISABLED_BY_POLICY_DEFAULT                              => "STATUS_ACCESS_DISABLED_BY_POLICY_DEFAULT",
        SMB_NTSTATUS_ACCESS_DISABLED_BY_POLICY_PATH                                 => "STATUS_ACCESS_DISABLED_BY_POLICY_PATH",
        SMB_NTSTATUS_ACCESS_DISABLED_BY_POLICY_PUBLISHER                            => "STATUS_ACCESS_DISABLED_BY_POLICY_PUBLISHER",
        SMB_NTSTATUS_ACCESS_DISABLED_BY_POLICY_OTHER                                => "STATUS_ACCESS_DISABLED_BY_POLICY_OTHER",
        SMB_NTSTATUS_FAILED_DRIVER_ENTRY                                            => "STATUS_FAILED_DRIVER_ENTRY",
        SMB_NTSTATUS_DEVICE_ENUMERATION_ERROR                                       => "STATUS_DEVICE_ENUMERATION_ERROR",
        SMB_NTSTATUS_MOUNT_POINT_NOT_RESOLVED                                       => "STATUS_MOUNT_POINT_NOT_RESOLVED",
        SMB_NTSTATUS_INVALID_DEVICE_OBJECT_PARAMETER                                => "STATUS_INVALID_DEVICE_OBJECT_PARAMETER",
        SMB_NTSTATUS_MCA_OCCURED                                                    => "STATUS_MCA_OCCURED",
        SMB_NTSTATUS_DRIVER_BLOCKED_CRITICAL                                        => "STATUS_DRIVER_BLOCKED_CRITICAL",
        SMB_NTSTATUS_DRIVER_BLOCKED                                                 => "STATUS_DRIVER_BLOCKED",
        SMB_NTSTATUS_DRIVER_DATABASE_ERROR                                          => "STATUS_DRIVER_DATABASE_ERROR",
        SMB_NTSTATUS_SYSTEM_HIVE_TOO_LARGE                                          => "STATUS_SYSTEM_HIVE_TOO_LARGE",
        SMB_NTSTATUS_INVALID_IMPORT_OF_NON_DLL                                      => "STATUS_INVALID_IMPORT_OF_NON_DLL",
        SMB_NTSTATUS_NO_SECRETS                                                     => "STATUS_NO_SECRETS",
        SMB_NTSTATUS_ACCESS_DISABLED_NO_SAFER_UI_BY_POLICY                          => "STATUS_ACCESS_DISABLED_NO_SAFER_UI_BY_POLICY",
        SMB_NTSTATUS_FAILED_STACK_SWITCH                                            => "STATUS_FAILED_STACK_SWITCH",
        SMB_NTSTATUS_HEAP_CORRUPTION                                                => "STATUS_HEAP_CORRUPTION",
        SMB_NTSTATUS_SMARTCARD_WRONG_PIN                                            => "STATUS_SMARTCARD_WRONG_PIN",
        SMB_NTSTATUS_SMARTCARD_CARD_BLOCKED                                         => "STATUS_SMARTCARD_CARD_BLOCKED",
        SMB_NTSTATUS_SMARTCARD_CARD_NOT_AUTHENTICATED                               => "STATUS_SMARTCARD_CARD_NOT_AUTHENTICATED",
        SMB_NTSTATUS_SMARTCARD_NO_CARD                                              => "STATUS_SMARTCARD_NO_CARD",
        SMB_NTSTATUS_SMARTCARD_NO_KEY_CONTAINER                                     => "STATUS_SMARTCARD_NO_KEY_CONTAINER",
        SMB_NTSTATUS_SMARTCARD_NO_CERTIFICATE                                       => "STATUS_SMARTCARD_NO_CERTIFICATE",
        SMB_NTSTATUS_SMARTCARD_NO_KEYSET                                            => "STATUS_SMARTCARD_NO_KEYSET",
        SMB_NTSTATUS_SMARTCARD_IO_ERROR                                             => "STATUS_SMARTCARD_IO_ERROR",
        SMB_NTSTATUS_DOWNGRADE_DETECTED                                             => "STATUS_DOWNGRADE_DETECTED",
        SMB_NTSTATUS_SMARTCARD_CERT_REVOKED                                         => "STATUS_SMARTCARD_CERT_REVOKED",
        SMB_NTSTATUS_ISSUING_CA_UNTRUSTED                                           => "STATUS_ISSUING_CA_UNTRUSTED",
        SMB_NTSTATUS_REVOCATION_OFFLINE_C                                           => "STATUS_REVOCATION_OFFLINE_C",
        SMB_NTSTATUS_PKINIT_CLIENT_FAILURE                                          => "STATUS_PKINIT_CLIENT_FAILURE",
        SMB_NTSTATUS_SMARTCARD_CERT_EXPIRED                                         => "STATUS_SMARTCARD_CERT_EXPIRED",
        SMB_NTSTATUS_DRIVER_FAILED_PRIOR_UNLOAD                                     => "STATUS_DRIVER_FAILED_PRIOR_UNLOAD",
        SMB_NTSTATUS_SMARTCARD_SILENT_CONTEXT                                       => "STATUS_SMARTCARD_SILENT_CONTEXT",
        SMB_NTSTATUS_PER_USER_TRUST_QUOTA_EXCEEDED                                  => "STATUS_PER_USER_TRUST_QUOTA_EXCEEDED",
        SMB_NTSTATUS_ALL_USER_TRUST_QUOTA_EXCEEDED                                  => "STATUS_ALL_USER_TRUST_QUOTA_EXCEEDED",
        SMB_NTSTATUS_USER_DELETE_TRUST_QUOTA_EXCEEDED                               => "STATUS_USER_DELETE_TRUST_QUOTA_EXCEEDED",
        SMB_NTSTATUS_DS_NAME_NOT_UNIQUE                                             => "STATUS_DS_NAME_NOT_UNIQUE",
        SMB_NTSTATUS_DS_DUPLICATE_ID_FOUND                                          => "STATUS_DS_DUPLICATE_ID_FOUND",
        SMB_NTSTATUS_DS_GROUP_CONVERSION_ERROR                                      => "STATUS_DS_GROUP_CONVERSION_ERROR",
        SMB_NTSTATUS_VOLSNAP_PREPARE_HIBERNATE                                      => "STATUS_VOLSNAP_PREPARE_HIBERNATE",
        SMB_NTSTATUS_USER2USER_REQUIRED                                             => "STATUS_USER2USER_REQUIRED",
        SMB_NTSTATUS_STACK_BUFFER_OVERRUN                                           => "STATUS_STACK_BUFFER_OVERRUN",
        SMB_NTSTATUS_NO_S4U_PROT_SUPPORT                                            => "STATUS_NO_S4U_PROT_SUPPORT",
        SMB_NTSTATUS_CROSSREALM_DELEGATION_FAILURE                                  => "STATUS_CROSSREALM_DELEGATION_FAILURE",
        SMB_NTSTATUS_REVOCATION_OFFLINE_KDC                                         => "STATUS_REVOCATION_OFFLINE_KDC",
        SMB_NTSTATUS_ISSUING_CA_UNTRUSTED_KDC                                       => "STATUS_ISSUING_CA_UNTRUSTED_KDC",
        SMB_NTSTATUS_KDC_CERT_EXPIRED                                               => "STATUS_KDC_CERT_EXPIRED",
        SMB_NTSTATUS_KDC_CERT_REVOKED                                               => "STATUS_KDC_CERT_REVOKED",
        SMB_NTSTATUS_PARAMETER_QUOTA_EXCEEDED                                       => "STATUS_PARAMETER_QUOTA_EXCEEDED",
        SMB_NTSTATUS_HIBERNATION_FAILURE                                            => "STATUS_HIBERNATION_FAILURE",
        SMB_NTSTATUS_DELAY_LOAD_FAILED                                              => "STATUS_DELAY_LOAD_FAILED",
        SMB_NTSTATUS_AUTHENTICATION_FIREWALL_FAILED                                 => "STATUS_AUTHENTICATION_FIREWALL_FAILED",
        SMB_NTSTATUS_VDM_DISALLOWED                                                 => "STATUS_VDM_DISALLOWED",
        SMB_NTSTATUS_HUNG_DISPLAY_DRIVER_THREAD                                     => "STATUS_HUNG_DISPLAY_DRIVER_THREAD",
        SMB_NTSTATUS_INSUFFICIENT_RESOURCE_FOR_SPECIFIED_SHARED_SECTION_SIZE        => "STATUS_INSUFFICIENT_RESOURCE_FOR_SPECIFIED_SHARED_SECTION_SIZE",
        SMB_NTSTATUS_INVALID_CRUNTIME_PARAMETER                                     => "STATUS_INVALID_CRUNTIME_PARAMETER",
        SMB_NTSTATUS_NTLM_BLOCKED                                                   => "STATUS_NTLM_BLOCKED",
        SMB_NTSTATUS_DS_SRC_SID_EXISTS_IN_FOREST                                    => "STATUS_DS_SRC_SID_EXISTS_IN_FOREST",
        SMB_NTSTATUS_DS_DOMAIN_NAME_EXISTS_IN_FOREST                                => "STATUS_DS_DOMAIN_NAME_EXISTS_IN_FOREST",
        SMB_NTSTATUS_DS_FLAT_NAME_EXISTS_IN_FOREST                                  => "STATUS_DS_FLAT_NAME_EXISTS_IN_FOREST",
        SMB_NTSTATUS_INVALID_USER_PRINCIPAL_NAME                                    => "STATUS_INVALID_USER_PRINCIPAL_NAME",
        SMB_NTSTATUS_ASSERTION_FAILURE                                              => "STATUS_ASSERTION_FAILURE",
        SMB_NTSTATUS_VERIFIER_STOP                                                  => "STATUS_VERIFIER_STOP",
        SMB_NTSTATUS_CALLBACK_POP_STACK                                             => "STATUS_CALLBACK_POP_STACK",
        SMB_NTSTATUS_INCOMPATIBLE_DRIVER_BLOCKED                                    => "STATUS_INCOMPATIBLE_DRIVER_BLOCKED",
        SMB_NTSTATUS_HIVE_UNLOADED                                                  => "STATUS_HIVE_UNLOADED",
        SMB_NTSTATUS_COMPRESSION_DISABLED                                           => "STATUS_COMPRESSION_DISABLED",
        SMB_NTSTATUS_FILE_SYSTEM_LIMITATION                                         => "STATUS_FILE_SYSTEM_LIMITATION",
        SMB_NTSTATUS_INVALID_IMAGE_HASH                                             => "STATUS_INVALID_IMAGE_HASH",
        SMB_NTSTATUS_NOT_CAPABLE                                                    => "STATUS_NOT_CAPABLE",
        SMB_NTSTATUS_REQUEST_OUT_OF_SEQUENCE                                        => "STATUS_REQUEST_OUT_OF_SEQUENCE",
        SMB_NTSTATUS_IMPLEMENTATION_LIMIT                                           => "STATUS_IMPLEMENTATION_LIMIT",
        SMB_NTSTATUS_ELEVATION_REQUIRED                                             => "STATUS_ELEVATION_REQUIRED",
        SMB_NTSTATUS_NO_SECURITY_CONTEXT                                            => "STATUS_NO_SECURITY_CONTEXT",
        SMB_NTSTATUS_PKU2U_CERT_FAILURE                                             => "STATUS_PKU2U_CERT_FAILURE",
        SMB_NTSTATUS_BEYOND_VDL                                                     => "STATUS_BEYOND_VDL",
        SMB_NTSTATUS_ENCOUNTERED_WRITE_IN_PROGRESS                                  => "STATUS_ENCOUNTERED_WRITE_IN_PROGRESS",
        SMB_NTSTATUS_PTE_CHANGED                                                    => "STATUS_PTE_CHANGED",
        SMB_NTSTATUS_PURGE_FAILED                                                   => "STATUS_PURGE_FAILED",
        SMB_NTSTATUS_CRED_REQUIRES_CONFIRMATION                                     => "STATUS_CRED_REQUIRES_CONFIRMATION",
        SMB_NTSTATUS_CS_ENCRYPTION_INVALID_SERVER_RESPONSE                          => "STATUS_CS_ENCRYPTION_INVALID_SERVER_RESPONSE",
        SMB_NTSTATUS_CS_ENCRYPTION_UNSUPPORTED_SERVER                               => "STATUS_CS_ENCRYPTION_UNSUPPORTED_SERVER",
        SMB_NTSTATUS_CS_ENCRYPTION_EXISTING_ENCRYPTED_FILE                          => "STATUS_CS_ENCRYPTION_EXISTING_ENCRYPTED_FILE",
        SMB_NTSTATUS_CS_ENCRYPTION_NEW_ENCRYPTED_FILE                               => "STATUS_CS_ENCRYPTION_NEW_ENCRYPTED_FILE",
        SMB_NTSTATUS_CS_ENCRYPTION_FILE_NOT_CSE                                     => "STATUS_CS_ENCRYPTION_FILE_NOT_CSE",
        SMB_NTSTATUS_INVALID_LABEL                                                  => "STATUS_INVALID_LABEL",
        SMB_NTSTATUS_DRIVER_PROCESS_TERMINATED                                      => "STATUS_DRIVER_PROCESS_TERMINATED",
        SMB_NTSTATUS_AMBIGUOUS_SYSTEM_DEVICE                                        => "STATUS_AMBIGUOUS_SYSTEM_DEVICE",
        SMB_NTSTATUS_SYSTEM_DEVICE_NOT_FOUND                                        => "STATUS_SYSTEM_DEVICE_NOT_FOUND",
        SMB_NTSTATUS_RESTART_BOOT_APPLICATION                                       => "STATUS_RESTART_BOOT_APPLICATION",
        SMB_NTSTATUS_INSUFFICIENT_NVRAM_RESOURCES                                   => "STATUS_INSUFFICIENT_NVRAM_RESOURCES",
        SMB_NTSTATUS_NO_RANGES_PROCESSED                                            => "STATUS_NO_RANGES_PROCESSED",
        SMB_NTSTATUS_DEVICE_FEATURE_NOT_SUPPORTED                                   => "STATUS_DEVICE_FEATURE_NOT_SUPPORTED",
        SMB_NTSTATUS_DEVICE_UNREACHABLE                                             => "STATUS_DEVICE_UNREACHABLE",
        SMB_NTSTATUS_INVALID_TOKEN                                                  => "STATUS_INVALID_TOKEN",
        SMB_NTSTATUS_SERVER_UNAVAILABLE                                             => "STATUS_SERVER_UNAVAILABLE",
        SMB_NTSTATUS_INVALID_TASK_NAME                                              => "STATUS_INVALID_TASK_NAME",
        SMB_NTSTATUS_INVALID_TASK_INDEX                                             => "STATUS_INVALID_TASK_INDEX",
        SMB_NTSTATUS_THREAD_ALREADY_IN_TASK                                         => "STATUS_THREAD_ALREADY_IN_TASK",
        SMB_NTSTATUS_CALLBACK_BYPASS                                                => "STATUS_CALLBACK_BYPASS",
        SMB_NTSTATUS_FAIL_FAST_EXCEPTION                                            => "STATUS_FAIL_FAST_EXCEPTION",
        SMB_NTSTATUS_IMAGE_CERT_REVOKED                                             => "STATUS_IMAGE_CERT_REVOKED",
        SMB_NTSTATUS_PORT_CLOSED                                                    => "STATUS_PORT_CLOSED",
        SMB_NTSTATUS_MESSAGE_LOST                                                   => "STATUS_MESSAGE_LOST",
        SMB_NTSTATUS_INVALID_MESSAGE                                                => "STATUS_INVALID_MESSAGE",
        SMB_NTSTATUS_REQUEST_CANCELED                                               => "STATUS_REQUEST_CANCELED",
        SMB_NTSTATUS_RECURSIVE_DISPATCH                                             => "STATUS_RECURSIVE_DISPATCH",
        SMB_NTSTATUS_LPC_RECEIVE_BUFFER_EXPECTED                                    => "STATUS_LPC_RECEIVE_BUFFER_EXPECTED",
        SMB_NTSTATUS_LPC_INVALID_CONNECTION_USAGE                                   => "STATUS_LPC_INVALID_CONNECTION_USAGE",
        SMB_NTSTATUS_LPC_REQUESTS_NOT_ALLOWED                                       => "STATUS_LPC_REQUESTS_NOT_ALLOWED",
        SMB_NTSTATUS_RESOURCE_IN_USE                                                => "STATUS_RESOURCE_IN_USE",
        SMB_NTSTATUS_HARDWARE_MEMORY_ERROR                                          => "STATUS_HARDWARE_MEMORY_ERROR",
        SMB_NTSTATUS_THREADPOOL_HANDLE_EXCEPTION                                    => "STATUS_THREADPOOL_HANDLE_EXCEPTION",
        SMB_NTSTATUS_THREADPOOL_SET_EVENT_ON_COMPLETION_FAILED                      => "STATUS_THREADPOOL_SET_EVENT_ON_COMPLETION_FAILED",
        SMB_NTSTATUS_THREADPOOL_RELEASE_SEMAPHORE_ON_COMPLETION_FAILED              => "STATUS_THREADPOOL_RELEASE_SEMAPHORE_ON_COMPLETION_FAILED",
        SMB_NTSTATUS_THREADPOOL_RELEASE_MUTEX_ON_COMPLETION_FAILED                  => "STATUS_THREADPOOL_RELEASE_MUTEX_ON_COMPLETION_FAILED",
        SMB_NTSTATUS_THREADPOOL_FREE_LIBRARY_ON_COMPLETION_FAILED                   => "STATUS_THREADPOOL_FREE_LIBRARY_ON_COMPLETION_FAILED",
        SMB_NTSTATUS_THREADPOOL_RELEASED_DURING_OPERATION                           => "STATUS_THREADPOOL_RELEASED_DURING_OPERATION",
        SMB_NTSTATUS_CALLBACK_RETURNED_WHILE_IMPERSONATING                          => "STATUS_CALLBACK_RETURNED_WHILE_IMPERSONATING",
        SMB_NTSTATUS_APC_RETURNED_WHILE_IMPERSONATING                               => "STATUS_APC_RETURNED_WHILE_IMPERSONATING",
        SMB_NTSTATUS_PROCESS_IS_PROTECTED                                           => "STATUS_PROCESS_IS_PROTECTED",
        SMB_NTSTATUS_MCA_EXCEPTION                                                  => "STATUS_MCA_EXCEPTION",
        SMB_NTSTATUS_CERTIFICATE_MAPPING_NOT_UNIQUE                                 => "STATUS_CERTIFICATE_MAPPING_NOT_UNIQUE",
        SMB_NTSTATUS_SYMLINK_CLASS_DISABLED                                         => "STATUS_SYMLINK_CLASS_DISABLED",
        SMB_NTSTATUS_INVALID_IDN_NORMALIZATION                                      => "STATUS_INVALID_IDN_NORMALIZATION",
        SMB_NTSTATUS_NO_UNICODE_TRANSLATION                                         => "STATUS_NO_UNICODE_TRANSLATION",
        SMB_NTSTATUS_ALREADY_REGISTERED                                             => "STATUS_ALREADY_REGISTERED",
        SMB_NTSTATUS_CONTEXT_MISMATCH                                               => "STATUS_CONTEXT_MISMATCH",
        SMB_NTSTATUS_PORT_ALREADY_HAS_COMPLETION_LIST                               => "STATUS_PORT_ALREADY_HAS_COMPLETION_LIST",
        SMB_NTSTATUS_CALLBACK_RETURNED_THREAD_PRIORITY                              => "STATUS_CALLBACK_RETURNED_THREAD_PRIORITY",
        SMB_NTSTATUS_INVALID_THREAD                                                 => "STATUS_INVALID_THREAD",
        SMB_NTSTATUS_CALLBACK_RETURNED_TRANSACTION                                  => "STATUS_CALLBACK_RETURNED_TRANSACTION",
        SMB_NTSTATUS_CALLBACK_RETURNED_LDR_LOCK                                     => "STATUS_CALLBACK_RETURNED_LDR_LOCK",
        SMB_NTSTATUS_CALLBACK_RETURNED_LANG                                         => "STATUS_CALLBACK_RETURNED_LANG",
        SMB_NTSTATUS_CALLBACK_RETURNED_PRI_BACK                                     => "STATUS_CALLBACK_RETURNED_PRI_BACK",
        SMB_NTSTATUS_DISK_REPAIR_DISABLED                                           => "STATUS_DISK_REPAIR_DISABLED",
        SMB_NTSTATUS_DS_DOMAIN_RENAME_IN_PROGRESS                                   => "STATUS_DS_DOMAIN_RENAME_IN_PROGRESS",
        SMB_NTSTATUS_DISK_QUOTA_EXCEEDED                                            => "STATUS_DISK_QUOTA_EXCEEDED",
        SMB_NTSTATUS_CONTENT_BLOCKED                                                => "STATUS_CONTENT_BLOCKED",
        SMB_NTSTATUS_BAD_CLUSTERS                                                   => "STATUS_BAD_CLUSTERS",
        SMB_NTSTATUS_VOLUME_DIRTY                                                   => "STATUS_VOLUME_DIRTY",
        SMB_NTSTATUS_FILE_CHECKED_OUT                                               => "STATUS_FILE_CHECKED_OUT",
        SMB_NTSTATUS_CHECKOUT_REQUIRED                                              => "STATUS_CHECKOUT_REQUIRED",
        SMB_NTSTATUS_BAD_FILE_TYPE                                                  => "STATUS_BAD_FILE_TYPE",
        SMB_NTSTATUS_FILE_TOO_LARGE                                                 => "STATUS_FILE_TOO_LARGE",
        SMB_NTSTATUS_FORMS_AUTH_REQUIRED                                            => "STATUS_FORMS_AUTH_REQUIRED",
        SMB_NTSTATUS_VIRUS_INFECTED                                                 => "STATUS_VIRUS_INFECTED",
        SMB_NTSTATUS_VIRUS_DELETED                                                  => "STATUS_VIRUS_DELETED",
        SMB_NTSTATUS_BAD_MCFG_TABLE                                                 => "STATUS_BAD_MCFG_TABLE",
        SMB_NTSTATUS_CANNOT_BREAK_OPLOCK                                            => "STATUS_CANNOT_BREAK_OPLOCK",
        SMB_NTSTATUS_WOW_ASSERTION                                                  => "STATUS_WOW_ASSERTION",
        SMB_NTSTATUS_INVALID_SIGNATURE                                              => "STATUS_INVALID_SIGNATURE",
        SMB_NTSTATUS_HMAC_NOT_SUPPORTED                                             => "STATUS_HMAC_NOT_SUPPORTED",
        SMB_NTSTATUS_IPSEC_QUEUE_OVERFLOW                                           => "STATUS_IPSEC_QUEUE_OVERFLOW",
        SMB_NTSTATUS_ND_QUEUE_OVERFLOW                                              => "STATUS_ND_QUEUE_OVERFLOW",
        SMB_NTSTATUS_HOPLIMIT_EXCEEDED                                              => "STATUS_HOPLIMIT_EXCEEDED",
        SMB_NTSTATUS_PROTOCOL_NOT_SUPPORTED                                         => "STATUS_PROTOCOL_NOT_SUPPORTED",
        SMB_NTSTATUS_LOST_WRITEBEHIND_DATA_NETWORK_DISCONNECTED                     => "STATUS_LOST_WRITEBEHIND_DATA_NETWORK_DISCONNECTED",
        SMB_NTSTATUS_LOST_WRITEBEHIND_DATA_NETWORK_SERVER_ERROR                     => "STATUS_LOST_WRITEBEHIND_DATA_NETWORK_SERVER_ERROR",
        SMB_NTSTATUS_LOST_WRITEBEHIND_DATA_LOCAL_DISK_ERROR                         => "STATUS_LOST_WRITEBEHIND_DATA_LOCAL_DISK_ERROR",
        SMB_NTSTATUS_XML_PARSE_ERROR                                                => "STATUS_XML_PARSE_ERROR",
        SMB_NTSTATUS_XMLDSIG_ERROR                                                  => "STATUS_XMLDSIG_ERROR",
        SMB_NTSTATUS_WRONG_COMPARTMENT                                              => "STATUS_WRONG_COMPARTMENT",
        SMB_NTSTATUS_AUTHIP_FAILURE                                                 => "STATUS_AUTHIP_FAILURE",
        SMB_NTSTATUS_DS_OID_MAPPED_GROUP_CANT_HAVE_MEMBERS                          => "STATUS_DS_OID_MAPPED_GROUP_CANT_HAVE_MEMBERS",
        SMB_NTSTATUS_DS_OID_NOT_FOUND                                               => "STATUS_DS_OID_NOT_FOUND",
        SMB_NTSTATUS_HASH_NOT_SUPPORTED                                             => "STATUS_HASH_NOT_SUPPORTED",
        SMB_NTSTATUS_HASH_NOT_PRESENT                                               => "STATUS_HASH_NOT_PRESENT",
        SMB_NTSTATUS_OFFLOAD_READ_FLT_NOT_SUPPORTED                                 => "STATUS_OFFLOAD_READ_FLT_NOT_SUPPORTED",
        SMB_NTSTATUS_OFFLOAD_WRITE_FLT_NOT_SUPPORTED                                => "STATUS_OFFLOAD_WRITE_FLT_NOT_SUPPORTED",
        SMB_NTSTATUS_OFFLOAD_READ_FILE_NOT_SUPPORTED                                => "STATUS_OFFLOAD_READ_FILE_NOT_SUPPORTED",
        SMB_NTSTATUS_OFFLOAD_WRITE_FILE_NOT_SUPPORTED                               => "STATUS_OFFLOAD_WRITE_FILE_NOT_SUPPORTED",
        SMB_NTDBG_NO_STATE_CHANGE                                                   => "DBG_NO_STATE_CHANGE",
        SMB_NTDBG_APP_NOT_IDLE                                                      => "DBG_APP_NOT_IDLE",
        SMB_NTRPC_NT_INVALID_STRING_BINDING                                         => "RPC_NT_INVALID_STRING_BINDING",
        SMB_NTRPC_NT_WRONG_KIND_OF_BINDING                                          => "RPC_NT_WRONG_KIND_OF_BINDING",
        SMB_NTRPC_NT_INVALID_BINDING                                                => "RPC_NT_INVALID_BINDING",
        SMB_NTRPC_NT_PROTSEQ_NOT_SUPPORTED                                          => "RPC_NT_PROTSEQ_NOT_SUPPORTED",
        SMB_NTRPC_NT_INVALID_RPC_PROTSEQ                                            => "RPC_NT_INVALID_RPC_PROTSEQ",
        SMB_NTRPC_NT_INVALID_STRING_UUID                                            => "RPC_NT_INVALID_STRING_UUID",
        SMB_NTRPC_NT_INVALID_ENDPOINT_FORMAT                                        => "RPC_NT_INVALID_ENDPOINT_FORMAT",
        SMB_NTRPC_NT_INVALID_NET_ADDR                                               => "RPC_NT_INVALID_NET_ADDR",
        SMB_NTRPC_NT_NO_ENDPOINT_FOUND                                              => "RPC_NT_NO_ENDPOINT_FOUND",
        SMB_NTRPC_NT_INVALID_TIMEOUT                                                => "RPC_NT_INVALID_TIMEOUT",
        SMB_NTRPC_NT_OBJECT_NOT_FOUND                                               => "RPC_NT_OBJECT_NOT_FOUND",
        SMB_NTRPC_NT_ALREADY_REGISTERED                                             => "RPC_NT_ALREADY_REGISTERED",
        SMB_NTRPC_NT_TYPE_ALREADY_REGISTERED                                        => "RPC_NT_TYPE_ALREADY_REGISTERED",
        SMB_NTRPC_NT_ALREADY_LISTENING                                              => "RPC_NT_ALREADY_LISTENING",
        SMB_NTRPC_NT_NO_PROTSEQS_REGISTERED                                         => "RPC_NT_NO_PROTSEQS_REGISTERED",
        SMB_NTRPC_NT_NOT_LISTENING                                                  => "RPC_NT_NOT_LISTENING",
        SMB_NTRPC_NT_UNKNOWN_MGR_TYPE                                               => "RPC_NT_UNKNOWN_MGR_TYPE",
        SMB_NTRPC_NT_UNKNOWN_IF                                                     => "RPC_NT_UNKNOWN_IF",
        SMB_NTRPC_NT_NO_BINDINGS                                                    => "RPC_NT_NO_BINDINGS",
        SMB_NTRPC_NT_NO_PROTSEQS                                                    => "RPC_NT_NO_PROTSEQS",
        SMB_NTRPC_NT_CANT_CREATE_ENDPOINT                                           => "RPC_NT_CANT_CREATE_ENDPOINT",
        SMB_NTRPC_NT_OUT_OF_RESOURCES                                               => "RPC_NT_OUT_OF_RESOURCES",
        SMB_NTRPC_NT_SERVER_UNAVAILABLE                                             => "RPC_NT_SERVER_UNAVAILABLE",
        SMB_NTRPC_NT_SERVER_TOO_BUSY                                                => "RPC_NT_SERVER_TOO_BUSY",
        SMB_NTRPC_NT_INVALID_NETWORK_OPTIONS                                        => "RPC_NT_INVALID_NETWORK_OPTIONS",
        SMB_NTRPC_NT_NO_CALL_ACTIVE                                                 => "RPC_NT_NO_CALL_ACTIVE",
        SMB_NTRPC_NT_CALL_FAILED                                                    => "RPC_NT_CALL_FAILED",
        SMB_NTRPC_NT_CALL_FAILED_DNE                                                => "RPC_NT_CALL_FAILED_DNE",
        SMB_NTRPC_NT_PROTOCOL_ERROR                                                 => "RPC_NT_PROTOCOL_ERROR",
        SMB_NTRPC_NT_UNSUPPORTED_TRANS_SYN                                          => "RPC_NT_UNSUPPORTED_TRANS_SYN",
        SMB_NTRPC_NT_UNSUPPORTED_TYPE                                               => "RPC_NT_UNSUPPORTED_TYPE",
        SMB_NTRPC_NT_INVALID_TAG                                                    => "RPC_NT_INVALID_TAG",
        SMB_NTRPC_NT_INVALID_BOUND                                                  => "RPC_NT_INVALID_BOUND",
        SMB_NTRPC_NT_NO_ENTRY_NAME                                                  => "RPC_NT_NO_ENTRY_NAME",
        SMB_NTRPC_NT_INVALID_NAME_SYNTAX                                            => "RPC_NT_INVALID_NAME_SYNTAX",
        SMB_NTRPC_NT_UNSUPPORTED_NAME_SYNTAX                                        => "RPC_NT_UNSUPPORTED_NAME_SYNTAX",
        SMB_NTRPC_NT_UUID_NO_ADDRESS                                                => "RPC_NT_UUID_NO_ADDRESS",
        SMB_NTRPC_NT_DUPLICATE_ENDPOINT                                             => "RPC_NT_DUPLICATE_ENDPOINT",
        SMB_NTRPC_NT_UNKNOWN_AUTHN_TYPE                                             => "RPC_NT_UNKNOWN_AUTHN_TYPE",
        SMB_NTRPC_NT_MAX_CALLS_TOO_SMALL                                            => "RPC_NT_MAX_CALLS_TOO_SMALL",
        SMB_NTRPC_NT_STRING_TOO_LONG                                                => "RPC_NT_STRING_TOO_LONG",
        SMB_NTRPC_NT_PROTSEQ_NOT_FOUND                                              => "RPC_NT_PROTSEQ_NOT_FOUND",
        SMB_NTRPC_NT_PROCNUM_OUT_OF_RANGE                                           => "RPC_NT_PROCNUM_OUT_OF_RANGE",
        SMB_NTRPC_NT_BINDING_HAS_NO_AUTH                                            => "RPC_NT_BINDING_HAS_NO_AUTH",
        SMB_NTRPC_NT_UNKNOWN_AUTHN_SERVICE                                          => "RPC_NT_UNKNOWN_AUTHN_SERVICE",
        SMB_NTRPC_NT_UNKNOWN_AUTHN_LEVEL                                            => "RPC_NT_UNKNOWN_AUTHN_LEVEL",
        SMB_NTRPC_NT_INVALID_AUTH_IDENTITY                                          => "RPC_NT_INVALID_AUTH_IDENTITY",
        SMB_NTRPC_NT_UNKNOWN_AUTHZ_SERVICE                                          => "RPC_NT_UNKNOWN_AUTHZ_SERVICE",
        SMB_NTEPT_NT_INVALID_ENTRY                                                  => "EPT_NT_INVALID_ENTRY",
        SMB_NTEPT_NT_CANT_PERFORM_OP                                                => "EPT_NT_CANT_PERFORM_OP",
        SMB_NTEPT_NT_NOT_REGISTERED                                                 => "EPT_NT_NOT_REGISTERED",
        SMB_NTRPC_NT_NOTHING_TO_EXPORT                                              => "RPC_NT_NOTHING_TO_EXPORT",
        SMB_NTRPC_NT_INCOMPLETE_NAME                                                => "RPC_NT_INCOMPLETE_NAME",
        SMB_NTRPC_NT_INVALID_VERS_OPTION                                            => "RPC_NT_INVALID_VERS_OPTION",
        SMB_NTRPC_NT_NO_MORE_MEMBERS                                                => "RPC_NT_NO_MORE_MEMBERS",
        SMB_NTRPC_NT_NOT_ALL_OBJS_UNEXPORTED                                        => "RPC_NT_NOT_ALL_OBJS_UNEXPORTED",
        SMB_NTRPC_NT_INTERFACE_NOT_FOUND                                            => "RPC_NT_INTERFACE_NOT_FOUND",
        SMB_NTRPC_NT_ENTRY_ALREADY_EXISTS                                           => "RPC_NT_ENTRY_ALREADY_EXISTS",
        SMB_NTRPC_NT_ENTRY_NOT_FOUND                                                => "RPC_NT_ENTRY_NOT_FOUND",
        SMB_NTRPC_NT_NAME_SERVICE_UNAVAILABLE                                       => "RPC_NT_NAME_SERVICE_UNAVAILABLE",
        SMB_NTRPC_NT_INVALID_NAF_ID                                                 => "RPC_NT_INVALID_NAF_ID",
        SMB_NTRPC_NT_CANNOT_SUPPORT                                                 => "RPC_NT_CANNOT_SUPPORT",
        SMB_NTRPC_NT_NO_CONTEXT_AVAILABLE                                           => "RPC_NT_NO_CONTEXT_AVAILABLE",
        SMB_NTRPC_NT_INTERNAL_ERROR                                                 => "RPC_NT_INTERNAL_ERROR",
        SMB_NTRPC_NT_ZERO_DIVIDE                                                    => "RPC_NT_ZERO_DIVIDE",
        SMB_NTRPC_NT_ADDRESS_ERROR                                                  => "RPC_NT_ADDRESS_ERROR",
        SMB_NTRPC_NT_FP_DIV_ZERO                                                    => "RPC_NT_FP_DIV_ZERO",
        SMB_NTRPC_NT_FP_UNDERFLOW                                                   => "RPC_NT_FP_UNDERFLOW",
        SMB_NTRPC_NT_FP_OVERFLOW                                                    => "RPC_NT_FP_OVERFLOW",
        SMB_NTRPC_NT_CALL_IN_PROGRESS                                               => "RPC_NT_CALL_IN_PROGRESS",
        SMB_NTRPC_NT_NO_MORE_BINDINGS                                               => "RPC_NT_NO_MORE_BINDINGS",
        SMB_NTRPC_NT_GROUP_MEMBER_NOT_FOUND                                         => "RPC_NT_GROUP_MEMBER_NOT_FOUND",
        SMB_NTEPT_NT_CANT_CREATE                                                    => "EPT_NT_CANT_CREATE",
        SMB_NTRPC_NT_INVALID_OBJECT                                                 => "RPC_NT_INVALID_OBJECT",
        SMB_NTRPC_NT_NO_INTERFACES                                                  => "RPC_NT_NO_INTERFACES",
        SMB_NTRPC_NT_CALL_CANCELLED                                                 => "RPC_NT_CALL_CANCELLED",
        SMB_NTRPC_NT_BINDING_INCOMPLETE                                             => "RPC_NT_BINDING_INCOMPLETE",
        SMB_NTRPC_NT_COMM_FAILURE                                                   => "RPC_NT_COMM_FAILURE",
        SMB_NTRPC_NT_UNSUPPORTED_AUTHN_LEVEL                                        => "RPC_NT_UNSUPPORTED_AUTHN_LEVEL",
        SMB_NTRPC_NT_NO_PRINC_NAME                                                  => "RPC_NT_NO_PRINC_NAME",
        SMB_NTRPC_NT_NOT_RPC_ERROR                                                  => "RPC_NT_NOT_RPC_ERROR",
        SMB_NTRPC_NT_SEC_PKG_ERROR                                                  => "RPC_NT_SEC_PKG_ERROR",
        SMB_NTRPC_NT_NOT_CANCELLED                                                  => "RPC_NT_NOT_CANCELLED",
        SMB_NTRPC_NT_INVALID_ASYNC_HANDLE                                           => "RPC_NT_INVALID_ASYNC_HANDLE",
        SMB_NTRPC_NT_INVALID_ASYNC_CALL                                             => "RPC_NT_INVALID_ASYNC_CALL",
        SMB_NTRPC_NT_PROXY_ACCESS_DENIED                                            => "RPC_NT_PROXY_ACCESS_DENIED",
        SMB_NTRPC_NT_NO_MORE_ENTRIES                                                => "RPC_NT_NO_MORE_ENTRIES",
        SMB_NTRPC_NT_SS_CHAR_TRANS_OPEN_FAIL                                        => "RPC_NT_SS_CHAR_TRANS_OPEN_FAIL",
        SMB_NTRPC_NT_SS_CHAR_TRANS_SHORT_FILE                                       => "RPC_NT_SS_CHAR_TRANS_SHORT_FILE",
        SMB_NTRPC_NT_SS_IN_NULL_CONTEXT                                             => "RPC_NT_SS_IN_NULL_CONTEXT",
        SMB_NTRPC_NT_SS_CONTEXT_MISMATCH                                            => "RPC_NT_SS_CONTEXT_MISMATCH",
        SMB_NTRPC_NT_SS_CONTEXT_DAMAGED                                             => "RPC_NT_SS_CONTEXT_DAMAGED",
        SMB_NTRPC_NT_SS_HANDLES_MISMATCH                                            => "RPC_NT_SS_HANDLES_MISMATCH",
        SMB_NTRPC_NT_SS_CANNOT_GET_CALL_HANDLE                                      => "RPC_NT_SS_CANNOT_GET_CALL_HANDLE",
        SMB_NTRPC_NT_NULL_REF_POINTER                                               => "RPC_NT_NULL_REF_POINTER",
        SMB_NTRPC_NT_ENUM_VALUE_OUT_OF_RANGE                                        => "RPC_NT_ENUM_VALUE_OUT_OF_RANGE",
        SMB_NTRPC_NT_BYTE_COUNT_TOO_SMALL                                           => "RPC_NT_BYTE_COUNT_TOO_SMALL",
        SMB_NTRPC_NT_BAD_STUB_DATA                                                  => "RPC_NT_BAD_STUB_DATA",
        SMB_NTRPC_NT_INVALID_ES_ACTION                                              => "RPC_NT_INVALID_ES_ACTION",
        SMB_NTRPC_NT_WRONG_ES_VERSION                                               => "RPC_NT_WRONG_ES_VERSION",
        SMB_NTRPC_NT_WRONG_STUB_VERSION                                             => "RPC_NT_WRONG_STUB_VERSION",
        SMB_NTRPC_NT_INVALID_PIPE_OBJECT                                            => "RPC_NT_INVALID_PIPE_OBJECT",
        SMB_NTRPC_NT_INVALID_PIPE_OPERATION                                         => "RPC_NT_INVALID_PIPE_OPERATION",
        SMB_NTRPC_NT_WRONG_PIPE_VERSION                                             => "RPC_NT_WRONG_PIPE_VERSION",
        SMB_NTRPC_NT_PIPE_CLOSED                                                    => "RPC_NT_PIPE_CLOSED",
        SMB_NTRPC_NT_PIPE_DISCIPLINE_ERROR                                          => "RPC_NT_PIPE_DISCIPLINE_ERROR",
        SMB_NTRPC_NT_PIPE_EMPTY                                                     => "RPC_NT_PIPE_EMPTY",
        SMB_NTSTATUS_PNP_BAD_MPS_TABLE                                              => "STATUS_PNP_BAD_MPS_TABLE",
        SMB_NTSTATUS_PNP_TRANSLATION_FAILED                                         => "STATUS_PNP_TRANSLATION_FAILED",
        SMB_NTSTATUS_PNP_IRQ_TRANSLATION_FAILED                                     => "STATUS_PNP_IRQ_TRANSLATION_FAILED",
        SMB_NTSTATUS_PNP_INVALID_ID                                                 => "STATUS_PNP_INVALID_ID",
        SMB_NTSTATUS_IO_REISSUE_AS_CACHED                                           => "STATUS_IO_REISSUE_AS_CACHED",
        SMB_NTSTATUS_CTX_WINSTATION_NAME_INVALID                                    => "STATUS_CTX_WINSTATION_NAME_INVALID",
        SMB_NTSTATUS_CTX_INVALID_PD                                                 => "STATUS_CTX_INVALID_PD",
        SMB_NTSTATUS_CTX_PD_NOT_FOUND                                               => "STATUS_CTX_PD_NOT_FOUND",
        SMB_NTSTATUS_CTX_CLOSE_PENDING                                              => "STATUS_CTX_CLOSE_PENDING",
        SMB_NTSTATUS_CTX_NO_OUTBUF                                                  => "STATUS_CTX_NO_OUTBUF",
        SMB_NTSTATUS_CTX_MODEM_INF_NOT_FOUND                                        => "STATUS_CTX_MODEM_INF_NOT_FOUND",
        SMB_NTSTATUS_CTX_INVALID_MODEMNAME                                          => "STATUS_CTX_INVALID_MODEMNAME",
        SMB_NTSTATUS_CTX_RESPONSE_ERROR                                             => "STATUS_CTX_RESPONSE_ERROR",
        SMB_NTSTATUS_CTX_MODEM_RESPONSE_TIMEOUT                                     => "STATUS_CTX_MODEM_RESPONSE_TIMEOUT",
        SMB_NTSTATUS_CTX_MODEM_RESPONSE_NO_CARRIER                                  => "STATUS_CTX_MODEM_RESPONSE_NO_CARRIER",
        SMB_NTSTATUS_CTX_MODEM_RESPONSE_NO_DIALTONE                                 => "STATUS_CTX_MODEM_RESPONSE_NO_DIALTONE",
        SMB_NTSTATUS_CTX_MODEM_RESPONSE_BUSY                                        => "STATUS_CTX_MODEM_RESPONSE_BUSY",
        SMB_NTSTATUS_CTX_MODEM_RESPONSE_VOICE                                       => "STATUS_CTX_MODEM_RESPONSE_VOICE",
        SMB_NTSTATUS_CTX_TD_ERROR                                                   => "STATUS_CTX_TD_ERROR",
        SMB_NTSTATUS_CTX_LICENSE_CLIENT_INVALID                                     => "STATUS_CTX_LICENSE_CLIENT_INVALID",
        SMB_NTSTATUS_CTX_LICENSE_NOT_AVAILABLE                                      => "STATUS_CTX_LICENSE_NOT_AVAILABLE",
        SMB_NTSTATUS_CTX_LICENSE_EXPIRED                                            => "STATUS_CTX_LICENSE_EXPIRED",
        SMB_NTSTATUS_CTX_WINSTATION_NOT_FOUND                                       => "STATUS_CTX_WINSTATION_NOT_FOUND",
        SMB_NTSTATUS_CTX_WINSTATION_NAME_COLLISION                                  => "STATUS_CTX_WINSTATION_NAME_COLLISION",
        SMB_NTSTATUS_CTX_WINSTATION_BUSY                                            => "STATUS_CTX_WINSTATION_BUSY",
        SMB_NTSTATUS_CTX_BAD_VIDEO_MODE                                             => "STATUS_CTX_BAD_VIDEO_MODE",
        SMB_NTSTATUS_CTX_GRAPHICS_INVALID                                           => "STATUS_CTX_GRAPHICS_INVALID",
        SMB_NTSTATUS_CTX_NOT_CONSOLE                                                => "STATUS_CTX_NOT_CONSOLE",
        SMB_NTSTATUS_CTX_CLIENT_QUERY_TIMEOUT                                       => "STATUS_CTX_CLIENT_QUERY_TIMEOUT",
        SMB_NTSTATUS_CTX_CONSOLE_DISCONNECT                                         => "STATUS_CTX_CONSOLE_DISCONNECT",
        SMB_NTSTATUS_CTX_CONSOLE_CONNECT                                            => "STATUS_CTX_CONSOLE_CONNECT",
        SMB_NTSTATUS_CTX_SHADOW_DENIED                                              => "STATUS_CTX_SHADOW_DENIED",
        SMB_NTSTATUS_CTX_WINSTATION_ACCESS_DENIED                                   => "STATUS_CTX_WINSTATION_ACCESS_DENIED",
        SMB_NTSTATUS_CTX_INVALID_WD                                                 => "STATUS_CTX_INVALID_WD",
        SMB_NTSTATUS_CTX_WD_NOT_FOUND                                               => "STATUS_CTX_WD_NOT_FOUND",
        SMB_NTSTATUS_CTX_SHADOW_INVALID                                             => "STATUS_CTX_SHADOW_INVALID",
        SMB_NTSTATUS_CTX_SHADOW_DISABLED                                            => "STATUS_CTX_SHADOW_DISABLED",
        SMB_NTSTATUS_RDP_PROTOCOL_ERROR                                             => "STATUS_RDP_PROTOCOL_ERROR",
        SMB_NTSTATUS_CTX_CLIENT_LICENSE_NOT_SET                                     => "STATUS_CTX_CLIENT_LICENSE_NOT_SET",
        SMB_NTSTATUS_CTX_CLIENT_LICENSE_IN_USE                                      => "STATUS_CTX_CLIENT_LICENSE_IN_USE",
        SMB_NTSTATUS_CTX_SHADOW_ENDED_BY_MODE_CHANGE                                => "STATUS_CTX_SHADOW_ENDED_BY_MODE_CHANGE",
        SMB_NTSTATUS_CTX_SHADOW_NOT_RUNNING                                         => "STATUS_CTX_SHADOW_NOT_RUNNING",
        SMB_NTSTATUS_CTX_LOGON_DISABLED                                             => "STATUS_CTX_LOGON_DISABLED",
        SMB_NTSTATUS_CTX_SECURITY_LAYER_ERROR                                       => "STATUS_CTX_SECURITY_LAYER_ERROR",
        SMB_NTSTATUS_TS_INCOMPATIBLE_SESSIONS                                       => "STATUS_TS_INCOMPATIBLE_SESSIONS",
        SMB_NTSTATUS_MUI_FILE_NOT_FOUND                                             => "STATUS_MUI_FILE_NOT_FOUND",
        SMB_NTSTATUS_MUI_INVALID_FILE                                               => "STATUS_MUI_INVALID_FILE",
        SMB_NTSTATUS_MUI_INVALID_RC_CONFIG                                          => "STATUS_MUI_INVALID_RC_CONFIG",
        SMB_NTSTATUS_MUI_INVALID_LOCALE_NAME                                        => "STATUS_MUI_INVALID_LOCALE_NAME",
        SMB_NTSTATUS_MUI_INVALID_ULTIMATEFALLBACK_NAME                              => "STATUS_MUI_INVALID_ULTIMATEFALLBACK_NAME",
        SMB_NTSTATUS_MUI_FILE_NOT_LOADED                                            => "STATUS_MUI_FILE_NOT_LOADED",
        SMB_NTSTATUS_RESOURCE_ENUM_USER_STOP                                        => "STATUS_RESOURCE_ENUM_USER_STOP",
        SMB_NTSTATUS_CLUSTER_INVALID_NODE                                           => "STATUS_CLUSTER_INVALID_NODE",
        SMB_NTSTATUS_CLUSTER_NODE_EXISTS                                            => "STATUS_CLUSTER_NODE_EXISTS",
        SMB_NTSTATUS_CLUSTER_JOIN_IN_PROGRESS                                       => "STATUS_CLUSTER_JOIN_IN_PROGRESS",
        SMB_NTSTATUS_CLUSTER_NODE_NOT_FOUND                                         => "STATUS_CLUSTER_NODE_NOT_FOUND",
        SMB_NTSTATUS_CLUSTER_LOCAL_NODE_NOT_FOUND                                   => "STATUS_CLUSTER_LOCAL_NODE_NOT_FOUND",
        SMB_NTSTATUS_CLUSTER_NETWORK_EXISTS                                         => "STATUS_CLUSTER_NETWORK_EXISTS",
        SMB_NTSTATUS_CLUSTER_NETWORK_NOT_FOUND                                      => "STATUS_CLUSTER_NETWORK_NOT_FOUND",
        SMB_NTSTATUS_CLUSTER_NETINTERFACE_EXISTS                                    => "STATUS_CLUSTER_NETINTERFACE_EXISTS",
        SMB_NTSTATUS_CLUSTER_NETINTERFACE_NOT_FOUND                                 => "STATUS_CLUSTER_NETINTERFACE_NOT_FOUND",
        SMB_NTSTATUS_CLUSTER_INVALID_REQUEST                                        => "STATUS_CLUSTER_INVALID_REQUEST",
        SMB_NTSTATUS_CLUSTER_INVALID_NETWORK_PROVIDER                               => "STATUS_CLUSTER_INVALID_NETWORK_PROVIDER",
        SMB_NTSTATUS_CLUSTER_NODE_DOWN                                              => "STATUS_CLUSTER_NODE_DOWN",
        SMB_NTSTATUS_CLUSTER_NODE_UNREACHABLE                                       => "STATUS_CLUSTER_NODE_UNREACHABLE",
        SMB_NTSTATUS_CLUSTER_NODE_NOT_MEMBER                                        => "STATUS_CLUSTER_NODE_NOT_MEMBER",
        SMB_NTSTATUS_CLUSTER_JOIN_NOT_IN_PROGRESS                                   => "STATUS_CLUSTER_JOIN_NOT_IN_PROGRESS",
        SMB_NTSTATUS_CLUSTER_INVALID_NETWORK                                        => "STATUS_CLUSTER_INVALID_NETWORK",
        SMB_NTSTATUS_CLUSTER_NO_NET_ADAPTERS                                        => "STATUS_CLUSTER_NO_NET_ADAPTERS",
        SMB_NTSTATUS_CLUSTER_NODE_UP                                                => "STATUS_CLUSTER_NODE_UP",
        SMB_NTSTATUS_CLUSTER_NODE_PAUSED                                            => "STATUS_CLUSTER_NODE_PAUSED",
        SMB_NTSTATUS_CLUSTER_NODE_NOT_PAUSED                                        => "STATUS_CLUSTER_NODE_NOT_PAUSED",
        SMB_NTSTATUS_CLUSTER_NO_SECURITY_CONTEXT                                    => "STATUS_CLUSTER_NO_SECURITY_CONTEXT",
        SMB_NTSTATUS_CLUSTER_NETWORK_NOT_INTERNAL                                   => "STATUS_CLUSTER_NETWORK_NOT_INTERNAL",
        SMB_NTSTATUS_CLUSTER_POISONED                                               => "STATUS_CLUSTER_POISONED",
        SMB_NTSTATUS_ACPI_INVALID_OPCODE                                            => "STATUS_ACPI_INVALID_OPCODE",
        SMB_NTSTATUS_ACPI_STACK_OVERFLOW                                            => "STATUS_ACPI_STACK_OVERFLOW",
        SMB_NTSTATUS_ACPI_ASSERT_FAILED                                             => "STATUS_ACPI_ASSERT_FAILED",
        SMB_NTSTATUS_ACPI_INVALID_INDEX                                             => "STATUS_ACPI_INVALID_INDEX",
        SMB_NTSTATUS_ACPI_INVALID_ARGUMENT                                          => "STATUS_ACPI_INVALID_ARGUMENT",
        SMB_NTSTATUS_ACPI_FATAL                                                     => "STATUS_ACPI_FATAL",
        SMB_NTSTATUS_ACPI_INVALID_SUPERNAME                                         => "STATUS_ACPI_INVALID_SUPERNAME",
        SMB_NTSTATUS_ACPI_INVALID_ARGTYPE                                           => "STATUS_ACPI_INVALID_ARGTYPE",
        SMB_NTSTATUS_ACPI_INVALID_OBJTYPE                                           => "STATUS_ACPI_INVALID_OBJTYPE",
        SMB_NTSTATUS_ACPI_INVALID_TARGETTYPE                                        => "STATUS_ACPI_INVALID_TARGETTYPE",
        SMB_NTSTATUS_ACPI_INCORRECT_ARGUMENT_COUNT                                  => "STATUS_ACPI_INCORRECT_ARGUMENT_COUNT",
        SMB_NTSTATUS_ACPI_ADDRESS_NOT_MAPPED                                        => "STATUS_ACPI_ADDRESS_NOT_MAPPED",
        SMB_NTSTATUS_ACPI_INVALID_EVENTTYPE                                         => "STATUS_ACPI_INVALID_EVENTTYPE",
        SMB_NTSTATUS_ACPI_HANDLER_COLLISION                                         => "STATUS_ACPI_HANDLER_COLLISION",
        SMB_NTSTATUS_ACPI_INVALID_DATA                                              => "STATUS_ACPI_INVALID_DATA",
        SMB_NTSTATUS_ACPI_INVALID_REGION                                            => "STATUS_ACPI_INVALID_REGION",
        SMB_NTSTATUS_ACPI_INVALID_ACCESS_SIZE                                       => "STATUS_ACPI_INVALID_ACCESS_SIZE",
        SMB_NTSTATUS_ACPI_ACQUIRE_GLOBAL_LOCK                                       => "STATUS_ACPI_ACQUIRE_GLOBAL_LOCK",
        SMB_NTSTATUS_ACPI_ALREADY_INITIALIZED                                       => "STATUS_ACPI_ALREADY_INITIALIZED",
        SMB_NTSTATUS_ACPI_NOT_INITIALIZED                                           => "STATUS_ACPI_NOT_INITIALIZED",
        SMB_NTSTATUS_ACPI_INVALID_MUTEX_LEVEL                                       => "STATUS_ACPI_INVALID_MUTEX_LEVEL",
        SMB_NTSTATUS_ACPI_MUTEX_NOT_OWNED                                           => "STATUS_ACPI_MUTEX_NOT_OWNED",
        SMB_NTSTATUS_ACPI_MUTEX_NOT_OWNER                                           => "STATUS_ACPI_MUTEX_NOT_OWNER",
        SMB_NTSTATUS_ACPI_RS_ACCESS                                                 => "STATUS_ACPI_RS_ACCESS",
        SMB_NTSTATUS_ACPI_INVALID_TABLE                                             => "STATUS_ACPI_INVALID_TABLE",
        SMB_NTSTATUS_ACPI_REG_HANDLER_FAILED                                        => "STATUS_ACPI_REG_HANDLER_FAILED",
        SMB_NTSTATUS_ACPI_POWER_REQUEST_FAILED                                      => "STATUS_ACPI_POWER_REQUEST_FAILED",
        SMB_NTSTATUS_SXS_SECTION_NOT_FOUND                                          => "STATUS_SXS_SECTION_NOT_FOUND",
        SMB_NTSTATUS_SXS_CANT_GEN_ACTCTX                                            => "STATUS_SXS_CANT_GEN_ACTCTX",
        SMB_NTSTATUS_SXS_INVALID_ACTCTXDATA_FORMAT                                  => "STATUS_SXS_INVALID_ACTCTXDATA_FORMAT",
        SMB_NTSTATUS_SXS_ASSEMBLY_NOT_FOUND                                         => "STATUS_SXS_ASSEMBLY_NOT_FOUND",
        SMB_NTSTATUS_SXS_MANIFEST_FORMAT_ERROR                                      => "STATUS_SXS_MANIFEST_FORMAT_ERROR",
        SMB_NTSTATUS_SXS_MANIFEST_PARSE_ERROR                                       => "STATUS_SXS_MANIFEST_PARSE_ERROR",
        SMB_NTSTATUS_SXS_ACTIVATION_CONTEXT_DISABLED                                => "STATUS_SXS_ACTIVATION_CONTEXT_DISABLED",
        SMB_NTSTATUS_SXS_KEY_NOT_FOUND                                              => "STATUS_SXS_KEY_NOT_FOUND",
        SMB_NTSTATUS_SXS_VERSION_CONFLICT                                           => "STATUS_SXS_VERSION_CONFLICT",
        SMB_NTSTATUS_SXS_WRONG_SECTION_TYPE                                         => "STATUS_SXS_WRONG_SECTION_TYPE",
        SMB_NTSTATUS_SXS_THREAD_QUERIES_DISABLED                                    => "STATUS_SXS_THREAD_QUERIES_DISABLED",
        SMB_NTSTATUS_SXS_ASSEMBLY_MISSING                                           => "STATUS_SXS_ASSEMBLY_MISSING",
        SMB_NTSTATUS_SXS_PROCESS_DEFAULT_ALREADY_SET                                => "STATUS_SXS_PROCESS_DEFAULT_ALREADY_SET",
        SMB_NTSTATUS_SXS_EARLY_DEACTIVATION                                         => "STATUS_SXS_EARLY_DEACTIVATION",
        SMB_NTSTATUS_SXS_INVALID_DEACTIVATION                                       => "STATUS_SXS_INVALID_DEACTIVATION",
        SMB_NTSTATUS_SXS_MULTIPLE_DEACTIVATION                                      => "STATUS_SXS_MULTIPLE_DEACTIVATION",
        SMB_NTSTATUS_SXS_SYSTEM_DEFAULT_ACTIVATION_CONTEXT_EMPTY                    => "STATUS_SXS_SYSTEM_DEFAULT_ACTIVATION_CONTEXT_EMPTY",
        SMB_NTSTATUS_SXS_PROCESS_TERMINATION_REQUESTED                              => "STATUS_SXS_PROCESS_TERMINATION_REQUESTED",
        SMB_NTSTATUS_SXS_CORRUPT_ACTIVATION_STACK                                   => "STATUS_SXS_CORRUPT_ACTIVATION_STACK",
        SMB_NTSTATUS_SXS_CORRUPTION                                                 => "STATUS_SXS_CORRUPTION",
        SMB_NTSTATUS_SXS_INVALID_IDENTITY_ATTRIBUTE_VALUE                           => "STATUS_SXS_INVALID_IDENTITY_ATTRIBUTE_VALUE",
        SMB_NTSTATUS_SXS_INVALID_IDENTITY_ATTRIBUTE_NAME                            => "STATUS_SXS_INVALID_IDENTITY_ATTRIBUTE_NAME",
        SMB_NTSTATUS_SXS_IDENTITY_DUPLICATE_ATTRIBUTE                               => "STATUS_SXS_IDENTITY_DUPLICATE_ATTRIBUTE",
        SMB_NTSTATUS_SXS_IDENTITY_PARSE_ERROR                                       => "STATUS_SXS_IDENTITY_PARSE_ERROR",
        SMB_NTSTATUS_SXS_COMPONENT_STORE_CORRUPT                                    => "STATUS_SXS_COMPONENT_STORE_CORRUPT",
        SMB_NTSTATUS_SXS_FILE_HASH_MISMATCH                                         => "STATUS_SXS_FILE_HASH_MISMATCH",
        SMB_NTSTATUS_SXS_MANIFEST_IDENTITY_SAME_BUT_CONTENTS_DIFFERENT              => "STATUS_SXS_MANIFEST_IDENTITY_SAME_BUT_CONTENTS_DIFFERENT",
        SMB_NTSTATUS_SXS_IDENTITIES_DIFFERENT                                       => "STATUS_SXS_IDENTITIES_DIFFERENT",
        SMB_NTSTATUS_SXS_ASSEMBLY_IS_NOT_A_DEPLOYMENT                               => "STATUS_SXS_ASSEMBLY_IS_NOT_A_DEPLOYMENT",
        SMB_NTSTATUS_SXS_FILE_NOT_PART_OF_ASSEMBLY                                  => "STATUS_SXS_FILE_NOT_PART_OF_ASSEMBLY",
        SMB_NTSTATUS_ADVANCED_INSTALLER_FAILED                                      => "STATUS_ADVANCED_INSTALLER_FAILED",
        SMB_NTSTATUS_XML_ENCODING_MISMATCH                                          => "STATUS_XML_ENCODING_MISMATCH",
        SMB_NTSTATUS_SXS_MANIFEST_TOO_BIG                                           => "STATUS_SXS_MANIFEST_TOO_BIG",
        SMB_NTSTATUS_SXS_SETTING_NOT_REGISTERED                                     => "STATUS_SXS_SETTING_NOT_REGISTERED",
        SMB_NTSTATUS_SXS_TRANSACTION_CLOSURE_INCOMPLETE                             => "STATUS_SXS_TRANSACTION_CLOSURE_INCOMPLETE",
        SMB_NTSTATUS_SMI_PRIMITIVE_INSTALLER_FAILED                                 => "STATUS_SMI_PRIMITIVE_INSTALLER_FAILED",
        SMB_NTSTATUS_GENERIC_COMMAND_FAILED                                         => "STATUS_GENERIC_COMMAND_FAILED",
        SMB_NTSTATUS_SXS_FILE_HASH_MISSING                                          => "STATUS_SXS_FILE_HASH_MISSING",
        SMB_NTSTATUS_TRANSACTIONAL_CONFLICT                                         => "STATUS_TRANSACTIONAL_CONFLICT",
        SMB_NTSTATUS_INVALID_TRANSACTION                                            => "STATUS_INVALID_TRANSACTION",
        SMB_NTSTATUS_TRANSACTION_NOT_ACTIVE                                         => "STATUS_TRANSACTION_NOT_ACTIVE",
        SMB_NTSTATUS_TM_INITIALIZATION_FAILED                                       => "STATUS_TM_INITIALIZATION_FAILED",
        SMB_NTSTATUS_RM_NOT_ACTIVE                                                  => "STATUS_RM_NOT_ACTIVE",
        SMB_NTSTATUS_RM_METADATA_CORRUPT                                            => "STATUS_RM_METADATA_CORRUPT",
        SMB_NTSTATUS_TRANSACTION_NOT_JOINED                                         => "STATUS_TRANSACTION_NOT_JOINED",
        SMB_NTSTATUS_DIRECTORY_NOT_RM                                               => "STATUS_DIRECTORY_NOT_RM",
        SMB_NTSTATUS_TRANSACTIONS_UNSUPPORTED_REMOTE                                => "STATUS_TRANSACTIONS_UNSUPPORTED_REMOTE",
        SMB_NTSTATUS_LOG_RESIZE_INVALID_SIZE                                        => "STATUS_LOG_RESIZE_INVALID_SIZE",
        SMB_NTSTATUS_REMOTE_FILE_VERSION_MISMATCH                                   => "STATUS_REMOTE_FILE_VERSION_MISMATCH",
        SMB_NTSTATUS_CRM_PROTOCOL_ALREADY_EXISTS                                    => "STATUS_CRM_PROTOCOL_ALREADY_EXISTS",
        SMB_NTSTATUS_TRANSACTION_PROPAGATION_FAILED                                 => "STATUS_TRANSACTION_PROPAGATION_FAILED",
        SMB_NTSTATUS_CRM_PROTOCOL_NOT_FOUND                                         => "STATUS_CRM_PROTOCOL_NOT_FOUND",
        SMB_NTSTATUS_TRANSACTION_SUPERIOR_EXISTS                                    => "STATUS_TRANSACTION_SUPERIOR_EXISTS",
        SMB_NTSTATUS_TRANSACTION_REQUEST_NOT_VALID                                  => "STATUS_TRANSACTION_REQUEST_NOT_VALID",
        SMB_NTSTATUS_TRANSACTION_NOT_REQUESTED                                      => "STATUS_TRANSACTION_NOT_REQUESTED",
        SMB_NTSTATUS_TRANSACTION_ALREADY_ABORTED                                    => "STATUS_TRANSACTION_ALREADY_ABORTED",
        SMB_NTSTATUS_TRANSACTION_ALREADY_COMMITTED                                  => "STATUS_TRANSACTION_ALREADY_COMMITTED",
        SMB_NTSTATUS_TRANSACTION_INVALID_MARSHALL_BUFFER                            => "STATUS_TRANSACTION_INVALID_MARSHALL_BUFFER",
        SMB_NTSTATUS_CURRENT_TRANSACTION_NOT_VALID                                  => "STATUS_CURRENT_TRANSACTION_NOT_VALID",
        SMB_NTSTATUS_LOG_GROWTH_FAILED                                              => "STATUS_LOG_GROWTH_FAILED",
        SMB_NTSTATUS_OBJECT_NO_LONGER_EXISTS                                        => "STATUS_OBJECT_NO_LONGER_EXISTS",
        SMB_NTSTATUS_STREAM_MINIVERSION_NOT_FOUND                                   => "STATUS_STREAM_MINIVERSION_NOT_FOUND",
        SMB_NTSTATUS_STREAM_MINIVERSION_NOT_VALID                                   => "STATUS_STREAM_MINIVERSION_NOT_VALID",
        SMB_NTSTATUS_MINIVERSION_INACCESSIBLE_FROM_SPECIFIED_TRANSACTION            => "STATUS_MINIVERSION_INACCESSIBLE_FROM_SPECIFIED_TRANSACTION",
        SMB_NTSTATUS_CANT_OPEN_MINIVERSION_WITH_MODIFY_INTENT                       => "STATUS_CANT_OPEN_MINIVERSION_WITH_MODIFY_INTENT",
        SMB_NTSTATUS_CANT_CREATE_MORE_STREAM_MINIVERSIONS                           => "STATUS_CANT_CREATE_MORE_STREAM_MINIVERSIONS",
        SMB_NTSTATUS_HANDLE_NO_LONGER_VALID                                         => "STATUS_HANDLE_NO_LONGER_VALID",
        SMB_NTSTATUS_LOG_CORRUPTION_DETECTED                                        => "STATUS_LOG_CORRUPTION_DETECTED",
        SMB_NTSTATUS_RM_DISCONNECTED                                                => "STATUS_RM_DISCONNECTED",
        SMB_NTSTATUS_ENLISTMENT_NOT_SUPERIOR                                        => "STATUS_ENLISTMENT_NOT_SUPERIOR",
        SMB_NTSTATUS_FILE_IDENTITY_NOT_PERSISTENT                                   => "STATUS_FILE_IDENTITY_NOT_PERSISTENT",
        SMB_NTSTATUS_CANT_BREAK_TRANSACTIONAL_DEPENDENCY                            => "STATUS_CANT_BREAK_TRANSACTIONAL_DEPENDENCY",
        SMB_NTSTATUS_CANT_CROSS_RM_BOUNDARY                                         => "STATUS_CANT_CROSS_RM_BOUNDARY",
        SMB_NTSTATUS_TXF_DIR_NOT_EMPTY                                              => "STATUS_TXF_DIR_NOT_EMPTY",
        SMB_NTSTATUS_INDOUBT_TRANSACTIONS_EXIST                                     => "STATUS_INDOUBT_TRANSACTIONS_EXIST",
        SMB_NTSTATUS_TM_VOLATILE                                                    => "STATUS_TM_VOLATILE",
        SMB_NTSTATUS_ROLLBACK_TIMER_EXPIRED                                         => "STATUS_ROLLBACK_TIMER_EXPIRED",
        SMB_NTSTATUS_TXF_ATTRIBUTE_CORRUPT                                          => "STATUS_TXF_ATTRIBUTE_CORRUPT",
        SMB_NTSTATUS_EFS_NOT_ALLOWED_IN_TRANSACTION                                 => "STATUS_EFS_NOT_ALLOWED_IN_TRANSACTION",
        SMB_NTSTATUS_TRANSACTIONAL_OPEN_NOT_ALLOWED                                 => "STATUS_TRANSACTIONAL_OPEN_NOT_ALLOWED",
        SMB_NTSTATUS_TRANSACTED_MAPPING_UNSUPPORTED_REMOTE                          => "STATUS_TRANSACTED_MAPPING_UNSUPPORTED_REMOTE",
        SMB_NTSTATUS_TRANSACTION_REQUIRED_PROMOTION                                 => "STATUS_TRANSACTION_REQUIRED_PROMOTION",
        SMB_NTSTATUS_CANNOT_EXECUTE_FILE_IN_TRANSACTION                             => "STATUS_CANNOT_EXECUTE_FILE_IN_TRANSACTION",
        SMB_NTSTATUS_TRANSACTIONS_NOT_FROZEN                                        => "STATUS_TRANSACTIONS_NOT_FROZEN",
        SMB_NTSTATUS_TRANSACTION_FREEZE_IN_PROGRESS                                 => "STATUS_TRANSACTION_FREEZE_IN_PROGRESS",
        SMB_NTSTATUS_NOT_SNAPSHOT_VOLUME                                            => "STATUS_NOT_SNAPSHOT_VOLUME",
        SMB_NTSTATUS_NO_SAVEPOINT_WITH_OPEN_FILES                                   => "STATUS_NO_SAVEPOINT_WITH_OPEN_FILES",
        SMB_NTSTATUS_SPARSE_NOT_ALLOWED_IN_TRANSACTION                              => "STATUS_SPARSE_NOT_ALLOWED_IN_TRANSACTION",
        SMB_NTSTATUS_TM_IDENTITY_MISMATCH                                           => "STATUS_TM_IDENTITY_MISMATCH",
        SMB_NTSTATUS_FLOATED_SECTION                                                => "STATUS_FLOATED_SECTION",
        SMB_NTSTATUS_CANNOT_ACCEPT_TRANSACTED_WORK                                  => "STATUS_CANNOT_ACCEPT_TRANSACTED_WORK",
        SMB_NTSTATUS_CANNOT_ABORT_TRANSACTIONS                                      => "STATUS_CANNOT_ABORT_TRANSACTIONS",
        SMB_NTSTATUS_TRANSACTION_NOT_FOUND                                          => "STATUS_TRANSACTION_NOT_FOUND",
        SMB_NTSTATUS_RESOURCEMANAGER_NOT_FOUND                                      => "STATUS_RESOURCEMANAGER_NOT_FOUND",
        SMB_NTSTATUS_ENLISTMENT_NOT_FOUND                                           => "STATUS_ENLISTMENT_NOT_FOUND",
        SMB_NTSTATUS_TRANSACTIONMANAGER_NOT_FOUND                                   => "STATUS_TRANSACTIONMANAGER_NOT_FOUND",
        SMB_NTSTATUS_TRANSACTIONMANAGER_NOT_ONLINE                                  => "STATUS_TRANSACTIONMANAGER_NOT_ONLINE",
        SMB_NTSTATUS_TRANSACTIONMANAGER_RECOVERY_NAME_COLLISION                     => "STATUS_TRANSACTIONMANAGER_RECOVERY_NAME_COLLISION",
        SMB_NTSTATUS_TRANSACTION_NOT_ROOT                                           => "STATUS_TRANSACTION_NOT_ROOT",
        SMB_NTSTATUS_TRANSACTION_OBJECT_EXPIRED                                     => "STATUS_TRANSACTION_OBJECT_EXPIRED",
        SMB_NTSTATUS_COMPRESSION_NOT_ALLOWED_IN_TRANSACTION                         => "STATUS_COMPRESSION_NOT_ALLOWED_IN_TRANSACTION",
        SMB_NTSTATUS_TRANSACTION_RESPONSE_NOT_ENLISTED                              => "STATUS_TRANSACTION_RESPONSE_NOT_ENLISTED",
        SMB_NTSTATUS_TRANSACTION_RECORD_TOO_LONG                                    => "STATUS_TRANSACTION_RECORD_TOO_LONG",
        SMB_NTSTATUS_NO_LINK_TRACKING_IN_TRANSACTION                                => "STATUS_NO_LINK_TRACKING_IN_TRANSACTION",
        SMB_NTSTATUS_OPERATION_NOT_SUPPORTED_IN_TRANSACTION                         => "STATUS_OPERATION_NOT_SUPPORTED_IN_TRANSACTION",
        SMB_NTSTATUS_TRANSACTION_INTEGRITY_VIOLATED                                 => "STATUS_TRANSACTION_INTEGRITY_VIOLATED",
        SMB_NTSTATUS_EXPIRED_HANDLE                                                 => "STATUS_EXPIRED_HANDLE",
        SMB_NTSTATUS_TRANSACTION_NOT_ENLISTED                                       => "STATUS_TRANSACTION_NOT_ENLISTED",
        SMB_NTSTATUS_LOG_SECTOR_INVALID                                             => "STATUS_LOG_SECTOR_INVALID",
        SMB_NTSTATUS_LOG_SECTOR_PARITY_INVALID                                      => "STATUS_LOG_SECTOR_PARITY_INVALID",
        SMB_NTSTATUS_LOG_SECTOR_REMAPPED                                            => "STATUS_LOG_SECTOR_REMAPPED",
        SMB_NTSTATUS_LOG_BLOCK_INCOMPLETE                                           => "STATUS_LOG_BLOCK_INCOMPLETE",
        SMB_NTSTATUS_LOG_INVALID_RANGE                                              => "STATUS_LOG_INVALID_RANGE",
        SMB_NTSTATUS_LOG_BLOCKS_EXHAUSTED                                           => "STATUS_LOG_BLOCKS_EXHAUSTED",
        SMB_NTSTATUS_LOG_READ_CONTEXT_INVALID                                       => "STATUS_LOG_READ_CONTEXT_INVALID",
        SMB_NTSTATUS_LOG_RESTART_INVALID                                            => "STATUS_LOG_RESTART_INVALID",
        SMB_NTSTATUS_LOG_BLOCK_VERSION                                              => "STATUS_LOG_BLOCK_VERSION",
        SMB_NTSTATUS_LOG_BLOCK_INVALID                                              => "STATUS_LOG_BLOCK_INVALID",
        SMB_NTSTATUS_LOG_READ_MODE_INVALID                                          => "STATUS_LOG_READ_MODE_INVALID",
        SMB_NTSTATUS_LOG_METADATA_CORRUPT                                           => "STATUS_LOG_METADATA_CORRUPT",
        SMB_NTSTATUS_LOG_METADATA_INVALID                                           => "STATUS_LOG_METADATA_INVALID",
        SMB_NTSTATUS_LOG_METADATA_INCONSISTENT                                      => "STATUS_LOG_METADATA_INCONSISTENT",
        SMB_NTSTATUS_LOG_RESERVATION_INVALID                                        => "STATUS_LOG_RESERVATION_INVALID",
        SMB_NTSTATUS_LOG_CANT_DELETE                                                => "STATUS_LOG_CANT_DELETE",
        SMB_NTSTATUS_LOG_CONTAINER_LIMIT_EXCEEDED                                   => "STATUS_LOG_CONTAINER_LIMIT_EXCEEDED",
        SMB_NTSTATUS_LOG_START_OF_LOG                                               => "STATUS_LOG_START_OF_LOG",
        SMB_NTSTATUS_LOG_POLICY_ALREADY_INSTALLED                                   => "STATUS_LOG_POLICY_ALREADY_INSTALLED",
        SMB_NTSTATUS_LOG_POLICY_NOT_INSTALLED                                       => "STATUS_LOG_POLICY_NOT_INSTALLED",
        SMB_NTSTATUS_LOG_POLICY_INVALID                                             => "STATUS_LOG_POLICY_INVALID",
        SMB_NTSTATUS_LOG_POLICY_CONFLICT                                            => "STATUS_LOG_POLICY_CONFLICT",
        SMB_NTSTATUS_LOG_PINNED_ARCHIVE_TAIL                                        => "STATUS_LOG_PINNED_ARCHIVE_TAIL",
        SMB_NTSTATUS_LOG_RECORD_NONEXISTENT                                         => "STATUS_LOG_RECORD_NONEXISTENT",
        SMB_NTSTATUS_LOG_RECORDS_RESERVED_INVALID                                   => "STATUS_LOG_RECORDS_RESERVED_INVALID",
        SMB_NTSTATUS_LOG_SPACE_RESERVED_INVALID                                     => "STATUS_LOG_SPACE_RESERVED_INVALID",
        SMB_NTSTATUS_LOG_TAIL_INVALID                                               => "STATUS_LOG_TAIL_INVALID",
        SMB_NTSTATUS_LOG_FULL                                                       => "STATUS_LOG_FULL",
        SMB_NTSTATUS_LOG_MULTIPLEXED                                                => "STATUS_LOG_MULTIPLEXED",
        SMB_NTSTATUS_LOG_DEDICATED                                                  => "STATUS_LOG_DEDICATED",
        SMB_NTSTATUS_LOG_ARCHIVE_NOT_IN_PROGRESS                                    => "STATUS_LOG_ARCHIVE_NOT_IN_PROGRESS",
        SMB_NTSTATUS_LOG_ARCHIVE_IN_PROGRESS                                        => "STATUS_LOG_ARCHIVE_IN_PROGRESS",
        SMB_NTSTATUS_LOG_EPHEMERAL                                                  => "STATUS_LOG_EPHEMERAL",
        SMB_NTSTATUS_LOG_NOT_ENOUGH_CONTAINERS                                      => "STATUS_LOG_NOT_ENOUGH_CONTAINERS",
        SMB_NTSTATUS_LOG_CLIENT_ALREADY_REGISTERED                                  => "STATUS_LOG_CLIENT_ALREADY_REGISTERED",
        SMB_NTSTATUS_LOG_CLIENT_NOT_REGISTERED                                      => "STATUS_LOG_CLIENT_NOT_REGISTERED",
        SMB_NTSTATUS_LOG_FULL_HANDLER_IN_PROGRESS                                   => "STATUS_LOG_FULL_HANDLER_IN_PROGRESS",
        SMB_NTSTATUS_LOG_CONTAINER_READ_FAILED                                      => "STATUS_LOG_CONTAINER_READ_FAILED",
        SMB_NTSTATUS_LOG_CONTAINER_WRITE_FAILED                                     => "STATUS_LOG_CONTAINER_WRITE_FAILED",
        SMB_NTSTATUS_LOG_CONTAINER_OPEN_FAILED                                      => "STATUS_LOG_CONTAINER_OPEN_FAILED",
        SMB_NTSTATUS_LOG_CONTAINER_STATE_INVALID                                    => "STATUS_LOG_CONTAINER_STATE_INVALID",
        SMB_NTSTATUS_LOG_STATE_INVALID                                              => "STATUS_LOG_STATE_INVALID",
        SMB_NTSTATUS_LOG_PINNED                                                     => "STATUS_LOG_PINNED",
        SMB_NTSTATUS_LOG_METADATA_FLUSH_FAILED                                      => "STATUS_LOG_METADATA_FLUSH_FAILED",
        SMB_NTSTATUS_LOG_INCONSISTENT_SECURITY                                      => "STATUS_LOG_INCONSISTENT_SECURITY",
        SMB_NTSTATUS_LOG_APPENDED_FLUSH_FAILED                                      => "STATUS_LOG_APPENDED_FLUSH_FAILED",
        SMB_NTSTATUS_LOG_PINNED_RESERVATION                                         => "STATUS_LOG_PINNED_RESERVATION",
        SMB_NTSTATUS_VIDEO_HUNG_DISPLAY_DRIVER_THREAD                               => "STATUS_VIDEO_HUNG_DISPLAY_DRIVER_THREAD",
        SMB_NTSTATUS_FLT_NO_HANDLER_DEFINED                                         => "STATUS_FLT_NO_HANDLER_DEFINED",
        SMB_NTSTATUS_FLT_CONTEXT_ALREADY_DEFINED                                    => "STATUS_FLT_CONTEXT_ALREADY_DEFINED",
        SMB_NTSTATUS_FLT_INVALID_ASYNCHRONOUS_REQUEST                               => "STATUS_FLT_INVALID_ASYNCHRONOUS_REQUEST",
        SMB_NTSTATUS_FLT_DISALLOW_FAST_IO                                           => "STATUS_FLT_DISALLOW_FAST_IO",
        SMB_NTSTATUS_FLT_INVALID_NAME_REQUEST                                       => "STATUS_FLT_INVALID_NAME_REQUEST",
        SMB_NTSTATUS_FLT_NOT_SAFE_TO_POST_OPERATION                                 => "STATUS_FLT_NOT_SAFE_TO_POST_OPERATION",
        SMB_NTSTATUS_FLT_NOT_INITIALIZED                                            => "STATUS_FLT_NOT_INITIALIZED",
        SMB_NTSTATUS_FLT_FILTER_NOT_READY                                           => "STATUS_FLT_FILTER_NOT_READY",
        SMB_NTSTATUS_FLT_POST_OPERATION_CLEANUP                                     => "STATUS_FLT_POST_OPERATION_CLEANUP",
        SMB_NTSTATUS_FLT_INTERNAL_ERROR                                             => "STATUS_FLT_INTERNAL_ERROR",
        SMB_NTSTATUS_FLT_DELETING_OBJECT                                            => "STATUS_FLT_DELETING_OBJECT",
        SMB_NTSTATUS_FLT_MUST_BE_NONPAGED_POOL                                      => "STATUS_FLT_MUST_BE_NONPAGED_POOL",
        SMB_NTSTATUS_FLT_DUPLICATE_ENTRY                                            => "STATUS_FLT_DUPLICATE_ENTRY",
        SMB_NTSTATUS_FLT_CBDQ_DISABLED                                              => "STATUS_FLT_CBDQ_DISABLED",
        SMB_NTSTATUS_FLT_DO_NOT_ATTACH                                              => "STATUS_FLT_DO_NOT_ATTACH",
        SMB_NTSTATUS_FLT_DO_NOT_DETACH                                              => "STATUS_FLT_DO_NOT_DETACH",
        SMB_NTSTATUS_FLT_INSTANCE_ALTITUDE_COLLISION                                => "STATUS_FLT_INSTANCE_ALTITUDE_COLLISION",
        SMB_NTSTATUS_FLT_INSTANCE_NAME_COLLISION                                    => "STATUS_FLT_INSTANCE_NAME_COLLISION",
        SMB_NTSTATUS_FLT_FILTER_NOT_FOUND                                           => "STATUS_FLT_FILTER_NOT_FOUND",
        SMB_NTSTATUS_FLT_VOLUME_NOT_FOUND                                           => "STATUS_FLT_VOLUME_NOT_FOUND",
        SMB_NTSTATUS_FLT_INSTANCE_NOT_FOUND                                         => "STATUS_FLT_INSTANCE_NOT_FOUND",
        SMB_NTSTATUS_FLT_CONTEXT_ALLOCATION_NOT_FOUND                               => "STATUS_FLT_CONTEXT_ALLOCATION_NOT_FOUND",
        SMB_NTSTATUS_FLT_INVALID_CONTEXT_REGISTRATION                               => "STATUS_FLT_INVALID_CONTEXT_REGISTRATION",
        SMB_NTSTATUS_FLT_NAME_CACHE_MISS                                            => "STATUS_FLT_NAME_CACHE_MISS",
        SMB_NTSTATUS_FLT_NO_DEVICE_OBJECT                                           => "STATUS_FLT_NO_DEVICE_OBJECT",
        SMB_NTSTATUS_FLT_VOLUME_ALREADY_MOUNTED                                     => "STATUS_FLT_VOLUME_ALREADY_MOUNTED",
        SMB_NTSTATUS_FLT_ALREADY_ENLISTED                                           => "STATUS_FLT_ALREADY_ENLISTED",
        SMB_NTSTATUS_FLT_CONTEXT_ALREADY_LINKED                                     => "STATUS_FLT_CONTEXT_ALREADY_LINKED",
        SMB_NTSTATUS_FLT_NO_WAITER_FOR_REPLY                                        => "STATUS_FLT_NO_WAITER_FOR_REPLY",
        SMB_NTSTATUS_MONITOR_NO_DESCRIPTOR                                          => "STATUS_MONITOR_NO_DESCRIPTOR",
        SMB_NTSTATUS_MONITOR_UNKNOWN_DESCRIPTOR_FORMAT                              => "STATUS_MONITOR_UNKNOWN_DESCRIPTOR_FORMAT",
        SMB_NTSTATUS_MONITOR_INVALID_DESCRIPTOR_CHECKSUM                            => "STATUS_MONITOR_INVALID_DESCRIPTOR_CHECKSUM",
        SMB_NTSTATUS_MONITOR_INVALID_STANDARD_TIMING_BLOCK                          => "STATUS_MONITOR_INVALID_STANDARD_TIMING_BLOCK",
        SMB_NTSTATUS_MONITOR_WMI_DATABLOCK_REGISTRATION_FAILED                      => "STATUS_MONITOR_WMI_DATABLOCK_REGISTRATION_FAILED",
        SMB_NTSTATUS_MONITOR_INVALID_SERIAL_NUMBER_MONDSC_BLOCK                     => "STATUS_MONITOR_INVALID_SERIAL_NUMBER_MONDSC_BLOCK",
        SMB_NTSTATUS_MONITOR_INVALID_USER_FRIENDLY_MONDSC_BLOCK                     => "STATUS_MONITOR_INVALID_USER_FRIENDLY_MONDSC_BLOCK",
        SMB_NTSTATUS_MONITOR_NO_MORE_DESCRIPTOR_DATA                                => "STATUS_MONITOR_NO_MORE_DESCRIPTOR_DATA",
        SMB_NTSTATUS_MONITOR_INVALID_DETAILED_TIMING_BLOCK                          => "STATUS_MONITOR_INVALID_DETAILED_TIMING_BLOCK",
        SMB_NTSTATUS_MONITOR_INVALID_MANUFACTURE_DATE                               => "STATUS_MONITOR_INVALID_MANUFACTURE_DATE",
        SMB_NTSTATUS_GRAPHICS_NOT_EXCLUSIVE_MODE_OWNER                              => "STATUS_GRAPHICS_NOT_EXCLUSIVE_MODE_OWNER",
        SMB_NTSTATUS_GRAPHICS_INSUFFICIENT_DMA_BUFFER                               => "STATUS_GRAPHICS_INSUFFICIENT_DMA_BUFFER",
        SMB_NTSTATUS_GRAPHICS_INVALID_DISPLAY_ADAPTER                               => "STATUS_GRAPHICS_INVALID_DISPLAY_ADAPTER",
        SMB_NTSTATUS_GRAPHICS_ADAPTER_WAS_RESET                                     => "STATUS_GRAPHICS_ADAPTER_WAS_RESET",
        SMB_NTSTATUS_GRAPHICS_INVALID_DRIVER_MODEL                                  => "STATUS_GRAPHICS_INVALID_DRIVER_MODEL",
        SMB_NTSTATUS_GRAPHICS_PRESENT_MODE_CHANGED                                  => "STATUS_GRAPHICS_PRESENT_MODE_CHANGED",
        SMB_NTSTATUS_GRAPHICS_PRESENT_OCCLUDED                                      => "STATUS_GRAPHICS_PRESENT_OCCLUDED",
        SMB_NTSTATUS_GRAPHICS_PRESENT_DENIED                                        => "STATUS_GRAPHICS_PRESENT_DENIED",
        SMB_NTSTATUS_GRAPHICS_CANNOTCOLORCONVERT                                    => "STATUS_GRAPHICS_CANNOTCOLORCONVERT",
        SMB_NTSTATUS_GRAPHICS_PRESENT_REDIRECTION_DISABLED                          => "STATUS_GRAPHICS_PRESENT_REDIRECTION_DISABLED",
        SMB_NTSTATUS_GRAPHICS_PRESENT_UNOCCLUDED                                    => "STATUS_GRAPHICS_PRESENT_UNOCCLUDED",
        SMB_NTSTATUS_GRAPHICS_NO_VIDEO_MEMORY                                       => "STATUS_GRAPHICS_NO_VIDEO_MEMORY",
        SMB_NTSTATUS_GRAPHICS_CANT_LOCK_MEMORY                                      => "STATUS_GRAPHICS_CANT_LOCK_MEMORY",
        SMB_NTSTATUS_GRAPHICS_ALLOCATION_BUSY                                       => "STATUS_GRAPHICS_ALLOCATION_BUSY",
        SMB_NTSTATUS_GRAPHICS_TOO_MANY_REFERENCES                                   => "STATUS_GRAPHICS_TOO_MANY_REFERENCES",
        SMB_NTSTATUS_GRAPHICS_TRY_AGAIN_LATER                                       => "STATUS_GRAPHICS_TRY_AGAIN_LATER",
        SMB_NTSTATUS_GRAPHICS_TRY_AGAIN_NOW                                         => "STATUS_GRAPHICS_TRY_AGAIN_NOW",
        SMB_NTSTATUS_GRAPHICS_ALLOCATION_INVALID                                    => "STATUS_GRAPHICS_ALLOCATION_INVALID",
        SMB_NTSTATUS_GRAPHICS_UNSWIZZLING_APERTURE_UNAVAILABLE                      => "STATUS_GRAPHICS_UNSWIZZLING_APERTURE_UNAVAILABLE",
        SMB_NTSTATUS_GRAPHICS_UNSWIZZLING_APERTURE_UNSUPPORTED                      => "STATUS_GRAPHICS_UNSWIZZLING_APERTURE_UNSUPPORTED",
        SMB_NTSTATUS_GRAPHICS_CANT_EVICT_PINNED_ALLOCATION                          => "STATUS_GRAPHICS_CANT_EVICT_PINNED_ALLOCATION",
        SMB_NTSTATUS_GRAPHICS_INVALID_ALLOCATION_USAGE                              => "STATUS_GRAPHICS_INVALID_ALLOCATION_USAGE",
        SMB_NTSTATUS_GRAPHICS_CANT_RENDER_LOCKED_ALLOCATION                         => "STATUS_GRAPHICS_CANT_RENDER_LOCKED_ALLOCATION",
        SMB_NTSTATUS_GRAPHICS_ALLOCATION_CLOSED                                     => "STATUS_GRAPHICS_ALLOCATION_CLOSED",
        SMB_NTSTATUS_GRAPHICS_INVALID_ALLOCATION_INSTANCE                           => "STATUS_GRAPHICS_INVALID_ALLOCATION_INSTANCE",
        SMB_NTSTATUS_GRAPHICS_INVALID_ALLOCATION_HANDLE                             => "STATUS_GRAPHICS_INVALID_ALLOCATION_HANDLE",
        SMB_NTSTATUS_GRAPHICS_WRONG_ALLOCATION_DEVICE                               => "STATUS_GRAPHICS_WRONG_ALLOCATION_DEVICE",
        SMB_NTSTATUS_GRAPHICS_ALLOCATION_CONTENT_LOST                               => "STATUS_GRAPHICS_ALLOCATION_CONTENT_LOST",
        SMB_NTSTATUS_GRAPHICS_GPU_EXCEPTION_ON_DEVICE                               => "STATUS_GRAPHICS_GPU_EXCEPTION_ON_DEVICE",
        SMB_NTSTATUS_GRAPHICS_INVALID_VIDPN_TOPOLOGY                                => "STATUS_GRAPHICS_INVALID_VIDPN_TOPOLOGY",
        SMB_NTSTATUS_GRAPHICS_VIDPN_TOPOLOGY_NOT_SUPPORTED                          => "STATUS_GRAPHICS_VIDPN_TOPOLOGY_NOT_SUPPORTED",
        SMB_NTSTATUS_GRAPHICS_VIDPN_TOPOLOGY_CURRENTLY_NOT_SUPPORTED                => "STATUS_GRAPHICS_VIDPN_TOPOLOGY_CURRENTLY_NOT_SUPPORTED",
        SMB_NTSTATUS_GRAPHICS_INVALID_VIDPN                                         => "STATUS_GRAPHICS_INVALID_VIDPN",
        SMB_NTSTATUS_GRAPHICS_INVALID_VIDEO_PRESENT_SOURCE                          => "STATUS_GRAPHICS_INVALID_VIDEO_PRESENT_SOURCE",
        SMB_NTSTATUS_GRAPHICS_INVALID_VIDEO_PRESENT_TARGET                          => "STATUS_GRAPHICS_INVALID_VIDEO_PRESENT_TARGET",
        SMB_NTSTATUS_GRAPHICS_VIDPN_MODALITY_NOT_SUPPORTED                          => "STATUS_GRAPHICS_VIDPN_MODALITY_NOT_SUPPORTED",
        SMB_NTSTATUS_GRAPHICS_INVALID_VIDPN_SOURCEMODESET                           => "STATUS_GRAPHICS_INVALID_VIDPN_SOURCEMODESET",
        SMB_NTSTATUS_GRAPHICS_INVALID_VIDPN_TARGETMODESET                           => "STATUS_GRAPHICS_INVALID_VIDPN_TARGETMODESET",
        SMB_NTSTATUS_GRAPHICS_INVALID_FREQUENCY                                     => "STATUS_GRAPHICS_INVALID_FREQUENCY",
        SMB_NTSTATUS_GRAPHICS_INVALID_ACTIVE_REGION                                 => "STATUS_GRAPHICS_INVALID_ACTIVE_REGION",
        SMB_NTSTATUS_GRAPHICS_INVALID_TOTAL_REGION                                  => "STATUS_GRAPHICS_INVALID_TOTAL_REGION",
        SMB_NTSTATUS_GRAPHICS_INVALID_VIDEO_PRESENT_SOURCE_MODE                     => "STATUS_GRAPHICS_INVALID_VIDEO_PRESENT_SOURCE_MODE",
        SMB_NTSTATUS_GRAPHICS_INVALID_VIDEO_PRESENT_TARGET_MODE                     => "STATUS_GRAPHICS_INVALID_VIDEO_PRESENT_TARGET_MODE",
        SMB_NTSTATUS_GRAPHICS_PINNED_MODE_MUST_REMAIN_IN_SET                        => "STATUS_GRAPHICS_PINNED_MODE_MUST_REMAIN_IN_SET",
        SMB_NTSTATUS_GRAPHICS_PATH_ALREADY_IN_TOPOLOGY                              => "STATUS_GRAPHICS_PATH_ALREADY_IN_TOPOLOGY",
        SMB_NTSTATUS_GRAPHICS_MODE_ALREADY_IN_MODESET                               => "STATUS_GRAPHICS_MODE_ALREADY_IN_MODESET",
        SMB_NTSTATUS_GRAPHICS_INVALID_VIDEOPRESENTSOURCESET                         => "STATUS_GRAPHICS_INVALID_VIDEOPRESENTSOURCESET",
        SMB_NTSTATUS_GRAPHICS_INVALID_VIDEOPRESENTTARGETSET                         => "STATUS_GRAPHICS_INVALID_VIDEOPRESENTTARGETSET",
        SMB_NTSTATUS_GRAPHICS_SOURCE_ALREADY_IN_SET                                 => "STATUS_GRAPHICS_SOURCE_ALREADY_IN_SET",
        SMB_NTSTATUS_GRAPHICS_TARGET_ALREADY_IN_SET                                 => "STATUS_GRAPHICS_TARGET_ALREADY_IN_SET",
        SMB_NTSTATUS_GRAPHICS_INVALID_VIDPN_PRESENT_PATH                            => "STATUS_GRAPHICS_INVALID_VIDPN_PRESENT_PATH",
        SMB_NTSTATUS_GRAPHICS_NO_RECOMMENDED_VIDPN_TOPOLOGY                         => "STATUS_GRAPHICS_NO_RECOMMENDED_VIDPN_TOPOLOGY",
        SMB_NTSTATUS_GRAPHICS_INVALID_MONITOR_FREQUENCYRANGESET                     => "STATUS_GRAPHICS_INVALID_MONITOR_FREQUENCYRANGESET",
        SMB_NTSTATUS_GRAPHICS_INVALID_MONITOR_FREQUENCYRANGE                        => "STATUS_GRAPHICS_INVALID_MONITOR_FREQUENCYRANGE",
        SMB_NTSTATUS_GRAPHICS_FREQUENCYRANGE_NOT_IN_SET                             => "STATUS_GRAPHICS_FREQUENCYRANGE_NOT_IN_SET",
        SMB_NTSTATUS_GRAPHICS_FREQUENCYRANGE_ALREADY_IN_SET                         => "STATUS_GRAPHICS_FREQUENCYRANGE_ALREADY_IN_SET",
        SMB_NTSTATUS_GRAPHICS_STALE_MODESET                                         => "STATUS_GRAPHICS_STALE_MODESET",
        SMB_NTSTATUS_GRAPHICS_INVALID_MONITOR_SOURCEMODESET                         => "STATUS_GRAPHICS_INVALID_MONITOR_SOURCEMODESET",
        SMB_NTSTATUS_GRAPHICS_INVALID_MONITOR_SOURCE_MODE                           => "STATUS_GRAPHICS_INVALID_MONITOR_SOURCE_MODE",
        SMB_NTSTATUS_GRAPHICS_NO_RECOMMENDED_FUNCTIONAL_VIDPN                       => "STATUS_GRAPHICS_NO_RECOMMENDED_FUNCTIONAL_VIDPN",
        SMB_NTSTATUS_GRAPHICS_MODE_ID_MUST_BE_UNIQUE                                => "STATUS_GRAPHICS_MODE_ID_MUST_BE_UNIQUE",
        SMB_NTSTATUS_GRAPHICS_EMPTY_ADAPTER_MONITOR_MODE_SUPPORT_INTERSECTION       => "STATUS_GRAPHICS_EMPTY_ADAPTER_MONITOR_MODE_SUPPORT_INTERSECTION",
        SMB_NTSTATUS_GRAPHICS_VIDEO_PRESENT_TARGETS_LESS_THAN_SOURCES               => "STATUS_GRAPHICS_VIDEO_PRESENT_TARGETS_LESS_THAN_SOURCES",
        SMB_NTSTATUS_GRAPHICS_PATH_NOT_IN_TOPOLOGY                                  => "STATUS_GRAPHICS_PATH_NOT_IN_TOPOLOGY",
        SMB_NTSTATUS_GRAPHICS_ADAPTER_MUST_HAVE_AT_LEAST_ONE_SOURCE                 => "STATUS_GRAPHICS_ADAPTER_MUST_HAVE_AT_LEAST_ONE_SOURCE",
        SMB_NTSTATUS_GRAPHICS_ADAPTER_MUST_HAVE_AT_LEAST_ONE_TARGET                 => "STATUS_GRAPHICS_ADAPTER_MUST_HAVE_AT_LEAST_ONE_TARGET",
        SMB_NTSTATUS_GRAPHICS_INVALID_MONITORDESCRIPTORSET                          => "STATUS_GRAPHICS_INVALID_MONITORDESCRIPTORSET",
        SMB_NTSTATUS_GRAPHICS_INVALID_MONITORDESCRIPTOR                             => "STATUS_GRAPHICS_INVALID_MONITORDESCRIPTOR",
        SMB_NTSTATUS_GRAPHICS_MONITORDESCRIPTOR_NOT_IN_SET                          => "STATUS_GRAPHICS_MONITORDESCRIPTOR_NOT_IN_SET",
        SMB_NTSTATUS_GRAPHICS_MONITORDESCRIPTOR_ALREADY_IN_SET                      => "STATUS_GRAPHICS_MONITORDESCRIPTOR_ALREADY_IN_SET",
        SMB_NTSTATUS_GRAPHICS_MONITORDESCRIPTOR_ID_MUST_BE_UNIQUE                   => "STATUS_GRAPHICS_MONITORDESCRIPTOR_ID_MUST_BE_UNIQUE",
        SMB_NTSTATUS_GRAPHICS_INVALID_VIDPN_TARGET_SUBSET_TYPE                      => "STATUS_GRAPHICS_INVALID_VIDPN_TARGET_SUBSET_TYPE",
        SMB_NTSTATUS_GRAPHICS_RESOURCES_NOT_RELATED                                 => "STATUS_GRAPHICS_RESOURCES_NOT_RELATED",
        SMB_NTSTATUS_GRAPHICS_SOURCE_ID_MUST_BE_UNIQUE                              => "STATUS_GRAPHICS_SOURCE_ID_MUST_BE_UNIQUE",
        SMB_NTSTATUS_GRAPHICS_TARGET_ID_MUST_BE_UNIQUE                              => "STATUS_GRAPHICS_TARGET_ID_MUST_BE_UNIQUE",
        SMB_NTSTATUS_GRAPHICS_NO_AVAILABLE_VIDPN_TARGET                             => "STATUS_GRAPHICS_NO_AVAILABLE_VIDPN_TARGET",
        SMB_NTSTATUS_GRAPHICS_MONITOR_COULD_NOT_BE_ASSOCIATED_WITH_ADAPTER          => "STATUS_GRAPHICS_MONITOR_COULD_NOT_BE_ASSOCIATED_WITH_ADAPTER",
        SMB_NTSTATUS_GRAPHICS_NO_VIDPNMGR                                           => "STATUS_GRAPHICS_NO_VIDPNMGR",
        SMB_NTSTATUS_GRAPHICS_NO_ACTIVE_VIDPN                                       => "STATUS_GRAPHICS_NO_ACTIVE_VIDPN",
        SMB_NTSTATUS_GRAPHICS_STALE_VIDPN_TOPOLOGY                                  => "STATUS_GRAPHICS_STALE_VIDPN_TOPOLOGY",
        SMB_NTSTATUS_GRAPHICS_MONITOR_NOT_CONNECTED                                 => "STATUS_GRAPHICS_MONITOR_NOT_CONNECTED",
        SMB_NTSTATUS_GRAPHICS_SOURCE_NOT_IN_TOPOLOGY                                => "STATUS_GRAPHICS_SOURCE_NOT_IN_TOPOLOGY",
        SMB_NTSTATUS_GRAPHICS_INVALID_PRIMARYSURFACE_SIZE                           => "STATUS_GRAPHICS_INVALID_PRIMARYSURFACE_SIZE",
        SMB_NTSTATUS_GRAPHICS_INVALID_VISIBLEREGION_SIZE                            => "STATUS_GRAPHICS_INVALID_VISIBLEREGION_SIZE",
        SMB_NTSTATUS_GRAPHICS_INVALID_STRIDE                                        => "STATUS_GRAPHICS_INVALID_STRIDE",
        SMB_NTSTATUS_GRAPHICS_INVALID_PIXELFORMAT                                   => "STATUS_GRAPHICS_INVALID_PIXELFORMAT",
        SMB_NTSTATUS_GRAPHICS_INVALID_COLORBASIS                                    => "STATUS_GRAPHICS_INVALID_COLORBASIS",
        SMB_NTSTATUS_GRAPHICS_INVALID_PIXELVALUEACCESSMODE                          => "STATUS_GRAPHICS_INVALID_PIXELVALUEACCESSMODE",
        SMB_NTSTATUS_GRAPHICS_TARGET_NOT_IN_TOPOLOGY                                => "STATUS_GRAPHICS_TARGET_NOT_IN_TOPOLOGY",
        SMB_NTSTATUS_GRAPHICS_NO_DISPLAY_MODE_MANAGEMENT_SUPPORT                    => "STATUS_GRAPHICS_NO_DISPLAY_MODE_MANAGEMENT_SUPPORT",
        SMB_NTSTATUS_GRAPHICS_VIDPN_SOURCE_IN_USE                                   => "STATUS_GRAPHICS_VIDPN_SOURCE_IN_USE",
        SMB_NTSTATUS_GRAPHICS_CANT_ACCESS_ACTIVE_VIDPN                              => "STATUS_GRAPHICS_CANT_ACCESS_ACTIVE_VIDPN",
        SMB_NTSTATUS_GRAPHICS_INVALID_PATH_IMPORTANCE_ORDINAL                       => "STATUS_GRAPHICS_INVALID_PATH_IMPORTANCE_ORDINAL",
        SMB_NTSTATUS_GRAPHICS_INVALID_PATH_CONTENT_GEOMETRY_TRANSFORMATION          => "STATUS_GRAPHICS_INVALID_PATH_CONTENT_GEOMETRY_TRANSFORMATION",
        SMB_NTSTATUS_GRAPHICS_PATH_CONTENT_GEOMETRY_TRANSFORMATION_NOT_SUPPORTED    => "STATUS_GRAPHICS_PATH_CONTENT_GEOMETRY_TRANSFORMATION_NOT_SUPPORTED",
        SMB_NTSTATUS_GRAPHICS_INVALID_GAMMA_RAMP                                    => "STATUS_GRAPHICS_INVALID_GAMMA_RAMP",
        SMB_NTSTATUS_GRAPHICS_GAMMA_RAMP_NOT_SUPPORTED                              => "STATUS_GRAPHICS_GAMMA_RAMP_NOT_SUPPORTED",
        SMB_NTSTATUS_GRAPHICS_MULTISAMPLING_NOT_SUPPORTED                           => "STATUS_GRAPHICS_MULTISAMPLING_NOT_SUPPORTED",
        SMB_NTSTATUS_GRAPHICS_MODE_NOT_IN_MODESET                                   => "STATUS_GRAPHICS_MODE_NOT_IN_MODESET",
        SMB_NTSTATUS_GRAPHICS_INVALID_VIDPN_TOPOLOGY_RECOMMENDATION_REASON          => "STATUS_GRAPHICS_INVALID_VIDPN_TOPOLOGY_RECOMMENDATION_REASON",
        SMB_NTSTATUS_GRAPHICS_INVALID_PATH_CONTENT_TYPE                             => "STATUS_GRAPHICS_INVALID_PATH_CONTENT_TYPE",
        SMB_NTSTATUS_GRAPHICS_INVALID_COPYPROTECTION_TYPE                           => "STATUS_GRAPHICS_INVALID_COPYPROTECTION_TYPE",
        SMB_NTSTATUS_GRAPHICS_UNASSIGNED_MODESET_ALREADY_EXISTS                     => "STATUS_GRAPHICS_UNASSIGNED_MODESET_ALREADY_EXISTS",
        SMB_NTSTATUS_GRAPHICS_INVALID_SCANLINE_ORDERING                             => "STATUS_GRAPHICS_INVALID_SCANLINE_ORDERING",
        SMB_NTSTATUS_GRAPHICS_TOPOLOGY_CHANGES_NOT_ALLOWED                          => "STATUS_GRAPHICS_TOPOLOGY_CHANGES_NOT_ALLOWED",
        SMB_NTSTATUS_GRAPHICS_NO_AVAILABLE_IMPORTANCE_ORDINALS                      => "STATUS_GRAPHICS_NO_AVAILABLE_IMPORTANCE_ORDINALS",
        SMB_NTSTATUS_GRAPHICS_INCOMPATIBLE_PRIVATE_FORMAT                           => "STATUS_GRAPHICS_INCOMPATIBLE_PRIVATE_FORMAT",
        SMB_NTSTATUS_GRAPHICS_INVALID_MODE_PRUNING_ALGORITHM                        => "STATUS_GRAPHICS_INVALID_MODE_PRUNING_ALGORITHM",
        SMB_NTSTATUS_GRAPHICS_INVALID_MONITOR_CAPABILITY_ORIGIN                     => "STATUS_GRAPHICS_INVALID_MONITOR_CAPABILITY_ORIGIN",
        SMB_NTSTATUS_GRAPHICS_INVALID_MONITOR_FREQUENCYRANGE_CONSTRAINT             => "STATUS_GRAPHICS_INVALID_MONITOR_FREQUENCYRANGE_CONSTRAINT",
        SMB_NTSTATUS_GRAPHICS_MAX_NUM_PATHS_REACHED                                 => "STATUS_GRAPHICS_MAX_NUM_PATHS_REACHED",
        SMB_NTSTATUS_GRAPHICS_CANCEL_VIDPN_TOPOLOGY_AUGMENTATION                    => "STATUS_GRAPHICS_CANCEL_VIDPN_TOPOLOGY_AUGMENTATION",
        SMB_NTSTATUS_GRAPHICS_INVALID_CLIENT_TYPE                                   => "STATUS_GRAPHICS_INVALID_CLIENT_TYPE",
        SMB_NTSTATUS_GRAPHICS_CLIENTVIDPN_NOT_SET                                   => "STATUS_GRAPHICS_CLIENTVIDPN_NOT_SET",
        SMB_NTSTATUS_GRAPHICS_SPECIFIED_CHILD_ALREADY_CONNECTED                     => "STATUS_GRAPHICS_SPECIFIED_CHILD_ALREADY_CONNECTED",
        SMB_NTSTATUS_GRAPHICS_CHILD_DESCRIPTOR_NOT_SUPPORTED                        => "STATUS_GRAPHICS_CHILD_DESCRIPTOR_NOT_SUPPORTED",
        SMB_NTSTATUS_GRAPHICS_NOT_A_LINKED_ADAPTER                                  => "STATUS_GRAPHICS_NOT_A_LINKED_ADAPTER",
        SMB_NTSTATUS_GRAPHICS_LEADLINK_NOT_ENUMERATED                               => "STATUS_GRAPHICS_LEADLINK_NOT_ENUMERATED",
        SMB_NTSTATUS_GRAPHICS_CHAINLINKS_NOT_ENUMERATED                             => "STATUS_GRAPHICS_CHAINLINKS_NOT_ENUMERATED",
        SMB_NTSTATUS_GRAPHICS_ADAPTER_CHAIN_NOT_READY                               => "STATUS_GRAPHICS_ADAPTER_CHAIN_NOT_READY",
        SMB_NTSTATUS_GRAPHICS_CHAINLINKS_NOT_STARTED                                => "STATUS_GRAPHICS_CHAINLINKS_NOT_STARTED",
        SMB_NTSTATUS_GRAPHICS_CHAINLINKS_NOT_POWERED_ON                             => "STATUS_GRAPHICS_CHAINLINKS_NOT_POWERED_ON",
        SMB_NTSTATUS_GRAPHICS_INCONSISTENT_DEVICE_LINK_STATE                        => "STATUS_GRAPHICS_INCONSISTENT_DEVICE_LINK_STATE",
        SMB_NTSTATUS_GRAPHICS_NOT_POST_DEVICE_DRIVER                                => "STATUS_GRAPHICS_NOT_POST_DEVICE_DRIVER",
        SMB_NTSTATUS_GRAPHICS_ADAPTER_ACCESS_NOT_EXCLUDED                           => "STATUS_GRAPHICS_ADAPTER_ACCESS_NOT_EXCLUDED",
        SMB_NTSTATUS_GRAPHICS_OPM_NOT_SUPPORTED                                     => "STATUS_GRAPHICS_OPM_NOT_SUPPORTED",
        SMB_NTSTATUS_GRAPHICS_COPP_NOT_SUPPORTED                                    => "STATUS_GRAPHICS_COPP_NOT_SUPPORTED",
        SMB_NTSTATUS_GRAPHICS_UAB_NOT_SUPPORTED                                     => "STATUS_GRAPHICS_UAB_NOT_SUPPORTED",
        SMB_NTSTATUS_GRAPHICS_OPM_INVALID_ENCRYPTED_PARAMETERS                      => "STATUS_GRAPHICS_OPM_INVALID_ENCRYPTED_PARAMETERS",
        SMB_NTSTATUS_GRAPHICS_OPM_PARAMETER_ARRAY_TOO_SMALL                         => "STATUS_GRAPHICS_OPM_PARAMETER_ARRAY_TOO_SMALL",
        SMB_NTSTATUS_GRAPHICS_OPM_NO_PROTECTED_OUTPUTS_EXIST                        => "STATUS_GRAPHICS_OPM_NO_PROTECTED_OUTPUTS_EXIST",
        SMB_NTSTATUS_GRAPHICS_PVP_NO_DISPLAY_DEVICE_CORRESPONDS_TO_NAME             => "STATUS_GRAPHICS_PVP_NO_DISPLAY_DEVICE_CORRESPONDS_TO_NAME",
        SMB_NTSTATUS_GRAPHICS_PVP_DISPLAY_DEVICE_NOT_ATTACHED_TO_DESKTOP            => "STATUS_GRAPHICS_PVP_DISPLAY_DEVICE_NOT_ATTACHED_TO_DESKTOP",
        SMB_NTSTATUS_GRAPHICS_PVP_MIRRORING_DEVICES_NOT_SUPPORTED                   => "STATUS_GRAPHICS_PVP_MIRRORING_DEVICES_NOT_SUPPORTED",
        SMB_NTSTATUS_GRAPHICS_OPM_INVALID_POINTER                                   => "STATUS_GRAPHICS_OPM_INVALID_POINTER",
        SMB_NTSTATUS_GRAPHICS_OPM_INTERNAL_ERROR                                    => "STATUS_GRAPHICS_OPM_INTERNAL_ERROR",
        SMB_NTSTATUS_GRAPHICS_OPM_INVALID_HANDLE                                    => "STATUS_GRAPHICS_OPM_INVALID_HANDLE",
        SMB_NTSTATUS_GRAPHICS_PVP_NO_MONITORS_CORRESPOND_TO_DISPLAY_DEVICE          => "STATUS_GRAPHICS_PVP_NO_MONITORS_CORRESPOND_TO_DISPLAY_DEVICE",
        SMB_NTSTATUS_GRAPHICS_PVP_INVALID_CERTIFICATE_LENGTH                        => "STATUS_GRAPHICS_PVP_INVALID_CERTIFICATE_LENGTH",
        SMB_NTSTATUS_GRAPHICS_OPM_SPANNING_MODE_ENABLED                             => "STATUS_GRAPHICS_OPM_SPANNING_MODE_ENABLED",
        SMB_NTSTATUS_GRAPHICS_OPM_THEATER_MODE_ENABLED                              => "STATUS_GRAPHICS_OPM_THEATER_MODE_ENABLED",
        SMB_NTSTATUS_GRAPHICS_PVP_HFS_FAILED                                        => "STATUS_GRAPHICS_PVP_HFS_FAILED",
        SMB_NTSTATUS_GRAPHICS_OPM_INVALID_SRM                                       => "STATUS_GRAPHICS_OPM_INVALID_SRM",
        SMB_NTSTATUS_GRAPHICS_OPM_OUTPUT_DOES_NOT_SUPPORT_HDCP                      => "STATUS_GRAPHICS_OPM_OUTPUT_DOES_NOT_SUPPORT_HDCP",
        SMB_NTSTATUS_GRAPHICS_OPM_OUTPUT_DOES_NOT_SUPPORT_ACP                       => "STATUS_GRAPHICS_OPM_OUTPUT_DOES_NOT_SUPPORT_ACP",
        SMB_NTSTATUS_GRAPHICS_OPM_OUTPUT_DOES_NOT_SUPPORT_CGMSA                     => "STATUS_GRAPHICS_OPM_OUTPUT_DOES_NOT_SUPPORT_CGMSA",
        SMB_NTSTATUS_GRAPHICS_OPM_HDCP_SRM_NEVER_SET                                => "STATUS_GRAPHICS_OPM_HDCP_SRM_NEVER_SET",
        SMB_NTSTATUS_GRAPHICS_OPM_RESOLUTION_TOO_HIGH                               => "STATUS_GRAPHICS_OPM_RESOLUTION_TOO_HIGH",
        SMB_NTSTATUS_GRAPHICS_OPM_ALL_HDCP_HARDWARE_ALREADY_IN_USE                  => "STATUS_GRAPHICS_OPM_ALL_HDCP_HARDWARE_ALREADY_IN_USE",
        SMB_NTSTATUS_GRAPHICS_OPM_PROTECTED_OUTPUT_NO_LONGER_EXISTS                 => "STATUS_GRAPHICS_OPM_PROTECTED_OUTPUT_NO_LONGER_EXISTS",
        SMB_NTSTATUS_GRAPHICS_OPM_SESSION_TYPE_CHANGE_IN_PROGRESS                   => "STATUS_GRAPHICS_OPM_SESSION_TYPE_CHANGE_IN_PROGRESS",
        SMB_NTSTATUS_GRAPHICS_OPM_PROTECTED_OUTPUT_DOES_NOT_HAVE_COPP_SEMANTICS     => "STATUS_GRAPHICS_OPM_PROTECTED_OUTPUT_DOES_NOT_HAVE_COPP_SEMANTICS",
        SMB_NTSTATUS_GRAPHICS_OPM_INVALID_INFORMATION_REQUEST                       => "STATUS_GRAPHICS_OPM_INVALID_INFORMATION_REQUEST",
        SMB_NTSTATUS_GRAPHICS_OPM_DRIVER_INTERNAL_ERROR                             => "STATUS_GRAPHICS_OPM_DRIVER_INTERNAL_ERROR",
        SMB_NTSTATUS_GRAPHICS_OPM_PROTECTED_OUTPUT_DOES_NOT_HAVE_OPM_SEMANTICS      => "STATUS_GRAPHICS_OPM_PROTECTED_OUTPUT_DOES_NOT_HAVE_OPM_SEMANTICS",
        SMB_NTSTATUS_GRAPHICS_OPM_SIGNALING_NOT_SUPPORTED                           => "STATUS_GRAPHICS_OPM_SIGNALING_NOT_SUPPORTED",
        SMB_NTSTATUS_GRAPHICS_OPM_INVALID_CONFIGURATION_REQUEST                     => "STATUS_GRAPHICS_OPM_INVALID_CONFIGURATION_REQUEST",
        SMB_NTSTATUS_GRAPHICS_I2C_NOT_SUPPORTED                                     => "STATUS_GRAPHICS_I2C_NOT_SUPPORTED",
        SMB_NTSTATUS_GRAPHICS_I2C_DEVICE_DOES_NOT_EXIST                             => "STATUS_GRAPHICS_I2C_DEVICE_DOES_NOT_EXIST",
        SMB_NTSTATUS_GRAPHICS_I2C_ERROR_TRANSMITTING_DATA                           => "STATUS_GRAPHICS_I2C_ERROR_TRANSMITTING_DATA",
        SMB_NTSTATUS_GRAPHICS_I2C_ERROR_RECEIVING_DATA                              => "STATUS_GRAPHICS_I2C_ERROR_RECEIVING_DATA",
        SMB_NTSTATUS_GRAPHICS_DDCCI_VCP_NOT_SUPPORTED                               => "STATUS_GRAPHICS_DDCCI_VCP_NOT_SUPPORTED",
        SMB_NTSTATUS_GRAPHICS_DDCCI_INVALID_DATA                                    => "STATUS_GRAPHICS_DDCCI_INVALID_DATA",
        SMB_NTSTATUS_GRAPHICS_DDCCI_MONITOR_RETURNED_INVALID_TIMING_STATUS_BYTE     => "STATUS_GRAPHICS_DDCCI_MONITOR_RETURNED_INVALID_TIMING_STATUS_BYTE",
        SMB_NTSTATUS_GRAPHICS_DDCCI_INVALID_CAPABILITIES_STRING                     => "STATUS_GRAPHICS_DDCCI_INVALID_CAPABILITIES_STRING",
        SMB_NTSTATUS_GRAPHICS_MCA_INTERNAL_ERROR                                    => "STATUS_GRAPHICS_MCA_INTERNAL_ERROR",
        SMB_NTSTATUS_GRAPHICS_DDCCI_INVALID_MESSAGE_COMMAND                         => "STATUS_GRAPHICS_DDCCI_INVALID_MESSAGE_COMMAND",
        SMB_NTSTATUS_GRAPHICS_DDCCI_INVALID_MESSAGE_LENGTH                          => "STATUS_GRAPHICS_DDCCI_INVALID_MESSAGE_LENGTH",
        SMB_NTSTATUS_GRAPHICS_DDCCI_INVALID_MESSAGE_CHECKSUM                        => "STATUS_GRAPHICS_DDCCI_INVALID_MESSAGE_CHECKSUM",
        SMB_NTSTATUS_GRAPHICS_INVALID_PHYSICAL_MONITOR_HANDLE                       => "STATUS_GRAPHICS_INVALID_PHYSICAL_MONITOR_HANDLE",
        SMB_NTSTATUS_GRAPHICS_MONITOR_NO_LONGER_EXISTS                              => "STATUS_GRAPHICS_MONITOR_NO_LONGER_EXISTS",
        SMB_NTSTATUS_GRAPHICS_ONLY_CONSOLE_SESSION_SUPPORTED                        => "STATUS_GRAPHICS_ONLY_CONSOLE_SESSION_SUPPORTED",
        SMB_NTSTATUS_GRAPHICS_NO_DISPLAY_DEVICE_CORRESPONDS_TO_NAME                 => "STATUS_GRAPHICS_NO_DISPLAY_DEVICE_CORRESPONDS_TO_NAME",
        SMB_NTSTATUS_GRAPHICS_DISPLAY_DEVICE_NOT_ATTACHED_TO_DESKTOP                => "STATUS_GRAPHICS_DISPLAY_DEVICE_NOT_ATTACHED_TO_DESKTOP",
        SMB_NTSTATUS_GRAPHICS_MIRRORING_DEVICES_NOT_SUPPORTED                       => "STATUS_GRAPHICS_MIRRORING_DEVICES_NOT_SUPPORTED",
        SMB_NTSTATUS_GRAPHICS_INVALID_POINTER                                       => "STATUS_GRAPHICS_INVALID_POINTER",
        SMB_NTSTATUS_GRAPHICS_NO_MONITORS_CORRESPOND_TO_DISPLAY_DEVICE              => "STATUS_GRAPHICS_NO_MONITORS_CORRESPOND_TO_DISPLAY_DEVICE",
        SMB_NTSTATUS_GRAPHICS_PARAMETER_ARRAY_TOO_SMALL                             => "STATUS_GRAPHICS_PARAMETER_ARRAY_TOO_SMALL",
        SMB_NTSTATUS_GRAPHICS_INTERNAL_ERROR                                        => "STATUS_GRAPHICS_INTERNAL_ERROR",
        SMB_NTSTATUS_GRAPHICS_SESSION_TYPE_CHANGE_IN_PROGRESS                       => "STATUS_GRAPHICS_SESSION_TYPE_CHANGE_IN_PROGRESS",
        SMB_NTSTATUS_FVE_LOCKED_VOLUME                                              => "STATUS_FVE_LOCKED_VOLUME",
        SMB_NTSTATUS_FVE_NOT_ENCRYPTED                                              => "STATUS_FVE_NOT_ENCRYPTED",
        SMB_NTSTATUS_FVE_BAD_INFORMATION                                            => "STATUS_FVE_BAD_INFORMATION",
        SMB_NTSTATUS_FVE_TOO_SMALL                                                  => "STATUS_FVE_TOO_SMALL",
        SMB_NTSTATUS_FVE_FAILED_WRONG_FS                                            => "STATUS_FVE_FAILED_WRONG_FS",
        SMB_NTSTATUS_FVE_FAILED_BAD_FS                                              => "STATUS_FVE_FAILED_BAD_FS",
        SMB_NTSTATUS_FVE_FS_NOT_EXTENDED                                            => "STATUS_FVE_FS_NOT_EXTENDED",
        SMB_NTSTATUS_FVE_FS_MOUNTED                                                 => "STATUS_FVE_FS_MOUNTED",
        SMB_NTSTATUS_FVE_NO_LICENSE                                                 => "STATUS_FVE_NO_LICENSE",
        SMB_NTSTATUS_FVE_ACTION_NOT_ALLOWED                                         => "STATUS_FVE_ACTION_NOT_ALLOWED",
        SMB_NTSTATUS_FVE_BAD_DATA                                                   => "STATUS_FVE_BAD_DATA",
        SMB_NTSTATUS_FVE_VOLUME_NOT_BOUND                                           => "STATUS_FVE_VOLUME_NOT_BOUND",
        SMB_NTSTATUS_FVE_NOT_DATA_VOLUME                                            => "STATUS_FVE_NOT_DATA_VOLUME",
        SMB_NTSTATUS_FVE_CONV_READ_ERROR                                            => "STATUS_FVE_CONV_READ_ERROR",
        SMB_NTSTATUS_FVE_CONV_WRITE_ERROR                                           => "STATUS_FVE_CONV_WRITE_ERROR",
        SMB_NTSTATUS_FVE_OVERLAPPED_UPDATE                                          => "STATUS_FVE_OVERLAPPED_UPDATE",
        SMB_NTSTATUS_FVE_FAILED_SECTOR_SIZE                                         => "STATUS_FVE_FAILED_SECTOR_SIZE",
        SMB_NTSTATUS_FVE_FAILED_AUTHENTICATION                                      => "STATUS_FVE_FAILED_AUTHENTICATION",
        SMB_NTSTATUS_FVE_NOT_OS_VOLUME                                              => "STATUS_FVE_NOT_OS_VOLUME",
        SMB_NTSTATUS_FVE_KEYFILE_NOT_FOUND                                          => "STATUS_FVE_KEYFILE_NOT_FOUND",
        SMB_NTSTATUS_FVE_KEYFILE_INVALID                                            => "STATUS_FVE_KEYFILE_INVALID",
        SMB_NTSTATUS_FVE_KEYFILE_NO_VMK                                             => "STATUS_FVE_KEYFILE_NO_VMK",
        SMB_NTSTATUS_FVE_TPM_DISABLED                                               => "STATUS_FVE_TPM_DISABLED",
        SMB_NTSTATUS_FVE_TPM_SRK_AUTH_NOT_ZERO                                      => "STATUS_FVE_TPM_SRK_AUTH_NOT_ZERO",
        SMB_NTSTATUS_FVE_TPM_INVALID_PCR                                            => "STATUS_FVE_TPM_INVALID_PCR",
        SMB_NTSTATUS_FVE_TPM_NO_VMK                                                 => "STATUS_FVE_TPM_NO_VMK",
        SMB_NTSTATUS_FVE_PIN_INVALID                                                => "STATUS_FVE_PIN_INVALID",
        SMB_NTSTATUS_FVE_AUTH_INVALID_APPLICATION                                   => "STATUS_FVE_AUTH_INVALID_APPLICATION",
        SMB_NTSTATUS_FVE_AUTH_INVALID_CONFIG                                        => "STATUS_FVE_AUTH_INVALID_CONFIG",
        SMB_NTSTATUS_FVE_DEBUGGER_ENABLED                                           => "STATUS_FVE_DEBUGGER_ENABLED",
        SMB_NTSTATUS_FVE_DRY_RUN_FAILED                                             => "STATUS_FVE_DRY_RUN_FAILED",
        SMB_NTSTATUS_FVE_BAD_METADATA_POINTER                                       => "STATUS_FVE_BAD_METADATA_POINTER",
        SMB_NTSTATUS_FVE_OLD_METADATA_COPY                                          => "STATUS_FVE_OLD_METADATA_COPY",
        SMB_NTSTATUS_FVE_REBOOT_REQUIRED                                            => "STATUS_FVE_REBOOT_REQUIRED",
        SMB_NTSTATUS_FVE_RAW_ACCESS                                                 => "STATUS_FVE_RAW_ACCESS",
        SMB_NTSTATUS_FVE_RAW_BLOCKED                                                => "STATUS_FVE_RAW_BLOCKED",
        SMB_NTSTATUS_FVE_NO_FEATURE_LICENSE                                         => "STATUS_FVE_NO_FEATURE_LICENSE",
        SMB_NTSTATUS_FVE_POLICY_USER_DISABLE_RDV_NOT_ALLOWED                        => "STATUS_FVE_POLICY_USER_DISABLE_RDV_NOT_ALLOWED",
        SMB_NTSTATUS_FVE_CONV_RECOVERY_FAILED                                       => "STATUS_FVE_CONV_RECOVERY_FAILED",
        SMB_NTSTATUS_FVE_VIRTUALIZED_SPACE_TOO_BIG                                  => "STATUS_FVE_VIRTUALIZED_SPACE_TOO_BIG",
        SMB_NTSTATUS_FVE_VOLUME_TOO_SMALL                                           => "STATUS_FVE_VOLUME_TOO_SMALL",
        SMB_NTSTATUS_FWP_CALLOUT_NOT_FOUND                                          => "STATUS_FWP_CALLOUT_NOT_FOUND",
        SMB_NTSTATUS_FWP_CONDITION_NOT_FOUND                                        => "STATUS_FWP_CONDITION_NOT_FOUND",
        SMB_NTSTATUS_FWP_FILTER_NOT_FOUND                                           => "STATUS_FWP_FILTER_NOT_FOUND",
        SMB_NTSTATUS_FWP_LAYER_NOT_FOUND                                            => "STATUS_FWP_LAYER_NOT_FOUND",
        SMB_NTSTATUS_FWP_PROVIDER_NOT_FOUND                                         => "STATUS_FWP_PROVIDER_NOT_FOUND",
        SMB_NTSTATUS_FWP_PROVIDER_CONTEXT_NOT_FOUND                                 => "STATUS_FWP_PROVIDER_CONTEXT_NOT_FOUND",
        SMB_NTSTATUS_FWP_SUBLAYER_NOT_FOUND                                         => "STATUS_FWP_SUBLAYER_NOT_FOUND",
        SMB_NTSTATUS_FWP_NOT_FOUND                                                  => "STATUS_FWP_NOT_FOUND",
        SMB_NTSTATUS_FWP_ALREADY_EXISTS                                             => "STATUS_FWP_ALREADY_EXISTS",
        SMB_NTSTATUS_FWP_IN_USE                                                     => "STATUS_FWP_IN_USE",
        SMB_NTSTATUS_FWP_DYNAMIC_SESSION_IN_PROGRESS                                => "STATUS_FWP_DYNAMIC_SESSION_IN_PROGRESS",
        SMB_NTSTATUS_FWP_WRONG_SESSION                                              => "STATUS_FWP_WRONG_SESSION",
        SMB_NTSTATUS_FWP_NO_TXN_IN_PROGRESS                                         => "STATUS_FWP_NO_TXN_IN_PROGRESS",
        SMB_NTSTATUS_FWP_TXN_IN_PROGRESS                                            => "STATUS_FWP_TXN_IN_PROGRESS",
        SMB_NTSTATUS_FWP_TXN_ABORTED                                                => "STATUS_FWP_TXN_ABORTED",
        SMB_NTSTATUS_FWP_SESSION_ABORTED                                            => "STATUS_FWP_SESSION_ABORTED",
        SMB_NTSTATUS_FWP_INCOMPATIBLE_TXN                                           => "STATUS_FWP_INCOMPATIBLE_TXN",
        SMB_NTSTATUS_FWP_TIMEOUT                                                    => "STATUS_FWP_TIMEOUT",
        SMB_NTSTATUS_FWP_NET_EVENTS_DISABLED                                        => "STATUS_FWP_NET_EVENTS_DISABLED",
        SMB_NTSTATUS_FWP_INCOMPATIBLE_LAYER                                         => "STATUS_FWP_INCOMPATIBLE_LAYER",
        SMB_NTSTATUS_FWP_KM_CLIENTS_ONLY                                            => "STATUS_FWP_KM_CLIENTS_ONLY",
        SMB_NTSTATUS_FWP_LIFETIME_MISMATCH                                          => "STATUS_FWP_LIFETIME_MISMATCH",
        SMB_NTSTATUS_FWP_BUILTIN_OBJECT                                             => "STATUS_FWP_BUILTIN_OBJECT",
        SMB_NTSTATUS_FWP_TOO_MANY_BOOTTIME_FILTERS                                  => "STATUS_FWP_TOO_MANY_BOOTTIME_FILTERS",
        SMB_NTSTATUS_FWP_NOTIFICATION_DROPPED                                       => "STATUS_FWP_NOTIFICATION_DROPPED",
        SMB_NTSTATUS_FWP_TRAFFIC_MISMATCH                                           => "STATUS_FWP_TRAFFIC_MISMATCH",
        SMB_NTSTATUS_FWP_INCOMPATIBLE_SA_STATE                                      => "STATUS_FWP_INCOMPATIBLE_SA_STATE",
        SMB_NTSTATUS_FWP_NULL_POINTER                                               => "STATUS_FWP_NULL_POINTER",
        SMB_NTSTATUS_FWP_INVALID_ENUMERATOR                                         => "STATUS_FWP_INVALID_ENUMERATOR",
        SMB_NTSTATUS_FWP_INVALID_FLAGS                                              => "STATUS_FWP_INVALID_FLAGS",
        SMB_NTSTATUS_FWP_INVALID_NET_MASK                                           => "STATUS_FWP_INVALID_NET_MASK",
        SMB_NTSTATUS_FWP_INVALID_RANGE                                              => "STATUS_FWP_INVALID_RANGE",
        SMB_NTSTATUS_FWP_INVALID_INTERVAL                                           => "STATUS_FWP_INVALID_INTERVAL",
        SMB_NTSTATUS_FWP_ZERO_LENGTH_ARRAY                                          => "STATUS_FWP_ZERO_LENGTH_ARRAY",
        SMB_NTSTATUS_FWP_NULL_DISPLAY_NAME                                          => "STATUS_FWP_NULL_DISPLAY_NAME",
        SMB_NTSTATUS_FWP_INVALID_ACTION_TYPE                                        => "STATUS_FWP_INVALID_ACTION_TYPE",
        SMB_NTSTATUS_FWP_INVALID_WEIGHT                                             => "STATUS_FWP_INVALID_WEIGHT",
        SMB_NTSTATUS_FWP_MATCH_TYPE_MISMATCH                                        => "STATUS_FWP_MATCH_TYPE_MISMATCH",
        SMB_NTSTATUS_FWP_TYPE_MISMATCH                                              => "STATUS_FWP_TYPE_MISMATCH",
        SMB_NTSTATUS_FWP_OUT_OF_BOUNDS                                              => "STATUS_FWP_OUT_OF_BOUNDS",
        SMB_NTSTATUS_FWP_RESERVED                                                   => "STATUS_FWP_RESERVED",
        SMB_NTSTATUS_FWP_DUPLICATE_CONDITION                                        => "STATUS_FWP_DUPLICATE_CONDITION",
        SMB_NTSTATUS_FWP_DUPLICATE_KEYMOD                                           => "STATUS_FWP_DUPLICATE_KEYMOD",
        SMB_NTSTATUS_FWP_ACTION_INCOMPATIBLE_WITH_LAYER                             => "STATUS_FWP_ACTION_INCOMPATIBLE_WITH_LAYER",
        SMB_NTSTATUS_FWP_ACTION_INCOMPATIBLE_WITH_SUBLAYER                          => "STATUS_FWP_ACTION_INCOMPATIBLE_WITH_SUBLAYER",
        SMB_NTSTATUS_FWP_CONTEXT_INCOMPATIBLE_WITH_LAYER                            => "STATUS_FWP_CONTEXT_INCOMPATIBLE_WITH_LAYER",
        SMB_NTSTATUS_FWP_CONTEXT_INCOMPATIBLE_WITH_CALLOUT                          => "STATUS_FWP_CONTEXT_INCOMPATIBLE_WITH_CALLOUT",
        SMB_NTSTATUS_FWP_INCOMPATIBLE_AUTH_METHOD                                   => "STATUS_FWP_INCOMPATIBLE_AUTH_METHOD",
        SMB_NTSTATUS_FWP_INCOMPATIBLE_DH_GROUP                                      => "STATUS_FWP_INCOMPATIBLE_DH_GROUP",
        SMB_NTSTATUS_FWP_EM_NOT_SUPPORTED                                           => "STATUS_FWP_EM_NOT_SUPPORTED",
        SMB_NTSTATUS_FWP_NEVER_MATCH                                                => "STATUS_FWP_NEVER_MATCH",
        SMB_NTSTATUS_FWP_PROVIDER_CONTEXT_MISMATCH                                  => "STATUS_FWP_PROVIDER_CONTEXT_MISMATCH",
        SMB_NTSTATUS_FWP_INVALID_PARAMETER                                          => "STATUS_FWP_INVALID_PARAMETER",
        SMB_NTSTATUS_FWP_TOO_MANY_SUBLAYERS                                         => "STATUS_FWP_TOO_MANY_SUBLAYERS",
        SMB_NTSTATUS_FWP_CALLOUT_NOTIFICATION_FAILED                                => "STATUS_FWP_CALLOUT_NOTIFICATION_FAILED",
        SMB_NTSTATUS_FWP_INCOMPATIBLE_AUTH_CONFIG                                   => "STATUS_FWP_INCOMPATIBLE_AUTH_CONFIG",
        SMB_NTSTATUS_FWP_INCOMPATIBLE_CIPHER_CONFIG                                 => "STATUS_FWP_INCOMPATIBLE_CIPHER_CONFIG",
        SMB_NTSTATUS_FWP_DUPLICATE_AUTH_METHOD                                      => "STATUS_FWP_DUPLICATE_AUTH_METHOD",
        SMB_NTSTATUS_FWP_TCPIP_NOT_READY                                            => "STATUS_FWP_TCPIP_NOT_READY",
        SMB_NTSTATUS_FWP_INJECT_HANDLE_CLOSING                                      => "STATUS_FWP_INJECT_HANDLE_CLOSING",
        SMB_NTSTATUS_FWP_INJECT_HANDLE_STALE                                        => "STATUS_FWP_INJECT_HANDLE_STALE",
        SMB_NTSTATUS_FWP_CANNOT_PEND                                                => "STATUS_FWP_CANNOT_PEND",
        SMB_NTSTATUS_NDIS_CLOSING                                                   => "STATUS_NDIS_CLOSING",
        SMB_NTSTATUS_NDIS_BAD_VERSION                                               => "STATUS_NDIS_BAD_VERSION",
        SMB_NTSTATUS_NDIS_BAD_CHARACTERISTICS                                       => "STATUS_NDIS_BAD_CHARACTERISTICS",
        SMB_NTSTATUS_NDIS_ADAPTER_NOT_FOUND                                         => "STATUS_NDIS_ADAPTER_NOT_FOUND",
        SMB_NTSTATUS_NDIS_OPEN_FAILED                                               => "STATUS_NDIS_OPEN_FAILED",
        SMB_NTSTATUS_NDIS_DEVICE_FAILED                                             => "STATUS_NDIS_DEVICE_FAILED",
        SMB_NTSTATUS_NDIS_MULTICAST_FULL                                            => "STATUS_NDIS_MULTICAST_FULL",
        SMB_NTSTATUS_NDIS_MULTICAST_EXISTS                                          => "STATUS_NDIS_MULTICAST_EXISTS",
        SMB_NTSTATUS_NDIS_MULTICAST_NOT_FOUND                                       => "STATUS_NDIS_MULTICAST_NOT_FOUND",
        SMB_NTSTATUS_NDIS_REQUEST_ABORTED                                           => "STATUS_NDIS_REQUEST_ABORTED",
        SMB_NTSTATUS_NDIS_RESET_IN_PROGRESS                                         => "STATUS_NDIS_RESET_IN_PROGRESS",
        SMB_NTSTATUS_NDIS_INVALID_PACKET                                            => "STATUS_NDIS_INVALID_PACKET",
        SMB_NTSTATUS_NDIS_INVALID_DEVICE_REQUEST                                    => "STATUS_NDIS_INVALID_DEVICE_REQUEST",
        SMB_NTSTATUS_NDIS_ADAPTER_NOT_READY                                         => "STATUS_NDIS_ADAPTER_NOT_READY",
        SMB_NTSTATUS_NDIS_INVALID_LENGTH                                            => "STATUS_NDIS_INVALID_LENGTH",
        SMB_NTSTATUS_NDIS_INVALID_DATA                                              => "STATUS_NDIS_INVALID_DATA",
        SMB_NTSTATUS_NDIS_BUFFER_TOO_SHORT                                          => "STATUS_NDIS_BUFFER_TOO_SHORT",
        SMB_NTSTATUS_NDIS_INVALID_OID                                               => "STATUS_NDIS_INVALID_OID",
        SMB_NTSTATUS_NDIS_ADAPTER_REMOVED                                           => "STATUS_NDIS_ADAPTER_REMOVED",
        SMB_NTSTATUS_NDIS_UNSUPPORTED_MEDIA                                         => "STATUS_NDIS_UNSUPPORTED_MEDIA",
        SMB_NTSTATUS_NDIS_GROUP_ADDRESS_IN_USE                                      => "STATUS_NDIS_GROUP_ADDRESS_IN_USE",
        SMB_NTSTATUS_NDIS_FILE_NOT_FOUND                                            => "STATUS_NDIS_FILE_NOT_FOUND",
        SMB_NTSTATUS_NDIS_ERROR_READING_FILE                                        => "STATUS_NDIS_ERROR_READING_FILE",
        SMB_NTSTATUS_NDIS_ALREADY_MAPPED                                            => "STATUS_NDIS_ALREADY_MAPPED",
        SMB_NTSTATUS_NDIS_RESOURCE_CONFLICT                                         => "STATUS_NDIS_RESOURCE_CONFLICT",
        SMB_NTSTATUS_NDIS_MEDIA_DISCONNECTED                                        => "STATUS_NDIS_MEDIA_DISCONNECTED",
        SMB_NTSTATUS_NDIS_INVALID_ADDRESS                                           => "STATUS_NDIS_INVALID_ADDRESS",
        SMB_NTSTATUS_NDIS_PAUSED                                                    => "STATUS_NDIS_PAUSED",
        SMB_NTSTATUS_NDIS_INTERFACE_NOT_FOUND                                       => "STATUS_NDIS_INTERFACE_NOT_FOUND",
        SMB_NTSTATUS_NDIS_UNSUPPORTED_REVISION                                      => "STATUS_NDIS_UNSUPPORTED_REVISION",
        SMB_NTSTATUS_NDIS_INVALID_PORT                                              => "STATUS_NDIS_INVALID_PORT",
        SMB_NTSTATUS_NDIS_INVALID_PORT_STATE                                        => "STATUS_NDIS_INVALID_PORT_STATE",
        SMB_NTSTATUS_NDIS_LOW_POWER_STATE                                           => "STATUS_NDIS_LOW_POWER_STATE",
        SMB_NTSTATUS_NDIS_NOT_SUPPORTED                                             => "STATUS_NDIS_NOT_SUPPORTED",
        SMB_NTSTATUS_NDIS_OFFLOAD_POLICY                                            => "STATUS_NDIS_OFFLOAD_POLICY",
        SMB_NTSTATUS_NDIS_OFFLOAD_CONNECTION_REJECTED                               => "STATUS_NDIS_OFFLOAD_CONNECTION_REJECTED",
        SMB_NTSTATUS_NDIS_OFFLOAD_PATH_REJECTED                                     => "STATUS_NDIS_OFFLOAD_PATH_REJECTED",
        SMB_NTSTATUS_NDIS_DOT11_AUTO_CONFIG_ENABLED                                 => "STATUS_NDIS_DOT11_AUTO_CONFIG_ENABLED",
        SMB_NTSTATUS_NDIS_DOT11_MEDIA_IN_USE                                        => "STATUS_NDIS_DOT11_MEDIA_IN_USE",
        SMB_NTSTATUS_NDIS_DOT11_POWER_STATE_INVALID                                 => "STATUS_NDIS_DOT11_POWER_STATE_INVALID",
        SMB_NTSTATUS_NDIS_PM_WOL_PATTERN_LIST_FULL                                  => "STATUS_NDIS_PM_WOL_PATTERN_LIST_FULL",
        SMB_NTSTATUS_NDIS_PM_PROTOCOL_OFFLOAD_LIST_FULL                             => "STATUS_NDIS_PM_PROTOCOL_OFFLOAD_LIST_FULL",
        SMB_NTSTATUS_IPSEC_BAD_SPI                                                  => "STATUS_IPSEC_BAD_SPI",
        SMB_NTSTATUS_IPSEC_SA_LIFETIME_EXPIRED                                      => "STATUS_IPSEC_SA_LIFETIME_EXPIRED",
        SMB_NTSTATUS_IPSEC_WRONG_SA                                                 => "STATUS_IPSEC_WRONG_SA",
        SMB_NTSTATUS_IPSEC_REPLAY_CHECK_FAILED                                      => "STATUS_IPSEC_REPLAY_CHECK_FAILED",
        SMB_NTSTATUS_IPSEC_INVALID_PACKET                                           => "STATUS_IPSEC_INVALID_PACKET",
        SMB_NTSTATUS_IPSEC_INTEGRITY_CHECK_FAILED                                   => "STATUS_IPSEC_INTEGRITY_CHECK_FAILED",
        SMB_NTSTATUS_IPSEC_CLEAR_TEXT_DROP                                          => "STATUS_IPSEC_CLEAR_TEXT_DROP",
        SMB_NTSTATUS_IPSEC_AUTH_FIREWALL_DROP                                       => "STATUS_IPSEC_AUTH_FIREWALL_DROP",
        SMB_NTSTATUS_IPSEC_THROTTLE_DROP                                            => "STATUS_IPSEC_THROTTLE_DROP",
        SMB_NTSTATUS_IPSEC_DOSP_BLOCK                                               => "STATUS_IPSEC_DOSP_BLOCK",
        SMB_NTSTATUS_IPSEC_DOSP_RECEIVED_MULTICAST                                  => "STATUS_IPSEC_DOSP_RECEIVED_MULTICAST",
        SMB_NTSTATUS_IPSEC_DOSP_INVALID_PACKET                                      => "STATUS_IPSEC_DOSP_INVALID_PACKET",
        SMB_NTSTATUS_IPSEC_DOSP_STATE_LOOKUP_FAILED                                 => "STATUS_IPSEC_DOSP_STATE_LOOKUP_FAILED",
        SMB_NTSTATUS_IPSEC_DOSP_MAX_ENTRIES                                         => "STATUS_IPSEC_DOSP_MAX_ENTRIES",
        SMB_NTSTATUS_IPSEC_DOSP_KEYMOD_NOT_ALLOWED                                  => "STATUS_IPSEC_DOSP_KEYMOD_NOT_ALLOWED",
        SMB_NTSTATUS_IPSEC_DOSP_MAX_PER_IP_RATELIMIT_QUEUES                         => "STATUS_IPSEC_DOSP_MAX_PER_IP_RATELIMIT_QUEUES",
        SMB_NTSTATUS_VOLMGR_MIRROR_NOT_SUPPORTED                                    => "STATUS_VOLMGR_MIRROR_NOT_SUPPORTED",
        SMB_NTSTATUS_VOLMGR_RAID5_NOT_SUPPORTED                                     => "STATUS_VOLMGR_RAID5_NOT_SUPPORTED",
        SMB_NTSTATUS_VIRTDISK_PROVIDER_NOT_FOUND                                    => "STATUS_VIRTDISK_PROVIDER_NOT_FOUND",
        SMB_NTSTATUS_VIRTDISK_NOT_VIRTUAL_DISK                                      => "STATUS_VIRTDISK_NOT_VIRTUAL_DISK",
        SMB_NTSTATUS_VHD_PARENT_VHD_ACCESS_DENIED                                   => "STATUS_VHD_PARENT_VHD_ACCESS_DENIED",
        SMB_NTSTATUS_VHD_CHILD_PARENT_SIZE_MISMATCH                                 => "STATUS_VHD_CHILD_PARENT_SIZE_MISMATCH",
        SMB_NTSTATUS_VHD_DIFFERENCING_CHAIN_CYCLE_DETECTED                          => "STATUS_VHD_DIFFERENCING_CHAIN_CYCLE_DETECTED",
        SMB_NTSTATUS_VHD_DIFFERENCING_CHAIN_ERROR_IN_PARENT                         => "STATUS_VHD_DIFFERENCING_CHAIN_ERROR_IN_PARENT",
 
        _ => { return (c).to_string(); },
    }.to_string()
}

pub const SMB_SRV_ERROR:                u16 = 1;
pub const SMB_SRV_BADPW:                u16 = 2;
pub const SMB_SRV_BADTYPE:              u16 = 3;
pub const SMB_SRV_ACCESS:               u16 = 4;
pub const SMB_SRV_BADUID:               u16 = 91;

pub fn smb_srv_error_string(c: u16) -> String {
    match c {
        SMB_SRV_ERROR           => "SRV_ERROR",
        SMB_SRV_BADPW           => "SRV_BADPW",
        SMB_SRV_BADTYPE         => "SRV_BADTYPE",
        SMB_SRV_ACCESS          => "SRV_ACCESS",
        SMB_SRV_BADUID          => "SRV_BADUID",
        _ => { return (c).to_string(); },
    }.to_string()
}

pub const SMB_DOS_SUCCESS:                u16 = 0;
pub const SMB_DOS_BAD_FUNC:               u16 = 1;
pub const SMB_DOS_BAD_FILE:               u16 = 2;
pub const SMB_DOS_BAD_PATH:               u16 = 3;
pub const SMB_DOS_TOO_MANY_OPEN_FILES:    u16 = 4;
pub const SMB_DOS_ACCESS_DENIED:          u16 = 5;

pub fn smb_dos_error_string(c: u16) -> String {
    match c {
        SMB_DOS_SUCCESS           => "DOS_SUCCESS",
        SMB_DOS_BAD_FUNC          => "DOS_BAD_FUNC",
        SMB_DOS_BAD_FILE          => "DOS_BAD_FILE",
        SMB_DOS_BAD_PATH          => "DOS_BAD_PATH",
        SMB_DOS_TOO_MANY_OPEN_FILES => "DOS_TOO_MANY_OPEN_FILES",
        SMB_DOS_ACCESS_DENIED     => "DOS_ACCESS_DENIED",
        _ => { return (c).to_string(); },
    }.to_string()
}

pub const NTLMSSP_NEGOTIATE:               u32 = 1;
pub const NTLMSSP_CHALLENGE:               u32 = 2;
pub const NTLMSSP_AUTH:                    u32 = 3;

pub fn ntlmssp_type_string(c: u32) -> String {
    match c {
        NTLMSSP_NEGOTIATE   => "NTLMSSP_NEGOTIATE",
        NTLMSSP_CHALLENGE   => "NTLMSSP_CHALLENGE",
        NTLMSSP_AUTH        => "NTLMSSP_AUTH",
        _ => { return (c).to_string(); },
    }.to_string()
}

#[derive(Default, Eq, PartialEq, Debug, Clone)]
pub struct SMBVerCmdStat {
    smb_ver: u8,
    smb1_cmd: u8,
    smb2_cmd: u16,

    status_set: bool,
    status_is_dos_error: bool,
    status_error_class: u8,
    status: u32,
}

impl SMBVerCmdStat {
    pub fn new() -> Self {
        Default::default()
    }
    pub fn new1(cmd: u8) -> Self {
        return Self {
            smb_ver: 1,
            smb1_cmd: cmd,
            ..Default::default()
        }
    }
    pub fn new1_with_ntstatus(cmd: u8, status: u32) -> Self {
        return Self {
            smb_ver: 1,
            smb1_cmd: cmd,
            status_set: true,
            status: status,
            ..Default::default()
        }
    }
    pub fn new2(cmd: u16) -> Self {
        return Self {
            smb_ver: 2,
            smb2_cmd: cmd,
            ..Default::default()
        }
    }

    pub fn new2_with_ntstatus(cmd: u16, status: u32) -> Self {
        return Self {
            smb_ver: 2,
            smb2_cmd: cmd,
            status_set: true,
            status: status,
            ..Default::default()
        }
    }

    pub fn set_smb1_cmd(&mut self, cmd: u8) -> bool {
        if self.smb_ver != 0 {
            return false;
        }
        self.smb_ver = 1;
        self.smb1_cmd = cmd;
        return true;
    }

    pub fn set_smb2_cmd(&mut self, cmd: u16) -> bool {
        if self.smb_ver != 0 {
            return false;
        }
        self.smb_ver = 2;
        self.smb2_cmd = cmd;
        return true;
    }

    pub fn get_version(&self) -> u8 {
        self.smb_ver
    }

    pub fn get_smb1_cmd(&self) -> (bool, u8) {
        if self.smb_ver != 1 {
            return (false, 0);
        }
        return (true, self.smb1_cmd);
    }

    pub fn get_smb2_cmd(&self) -> (bool, u16) {
        if self.smb_ver != 2 {
            return (false, 0);
        }
        return (true, self.smb2_cmd);
    }

    pub fn get_ntstatus(&self) -> (bool, u32) {
        (self.status_set && !self.status_is_dos_error, self.status)
    }

    pub fn get_dos_error(&self) -> (bool, u8, u16) {
        (self.status_set && self.status_is_dos_error, self.status_error_class, self.status as u16)
    }

    fn set_status(&mut self, status: u32, is_dos_error: bool)
    {
        if is_dos_error {
            self.status_is_dos_error = true;
            self.status_error_class = (status & 0x0000_00ff) as u8;
            self.status = (status & 0xffff_0000) >> 16;
        } else {
            self.status = status;
        }
        self.status_set = true;
    }

    pub fn set_ntstatus(&mut self, status: u32)
    {
        self.set_status(status, false)
    }

    pub fn set_status_dos_error(&mut self, status: u32)
    {
        self.set_status(status, true)
    }
}

/// "The FILETIME structure is a 64-bit value that represents the number of
/// 100-nanosecond intervals that have elapsed since January 1, 1601,
/// Coordinated Universal Time (UTC)."
#[derive(Eq, PartialEq, Debug, Clone)]
pub struct SMBFiletime {
    ts: u64, 
}

impl SMBFiletime {
    pub fn new(raw: u64) -> Self {
        Self {
            ts: raw,
        }
    }

    /// inspired by Bro, convert FILETIME to secs since unix epoch
    pub fn as_unix(&self) -> u32 {
        if self.ts > 116_444_736_000_000_000_u64 {
            let ts = self.ts / 10000000 - 11644473600;
            ts as u32
        } else {
            0
        }
    }
}

#[derive(Debug)]
pub enum SMBTransactionTypeData {
    FILE(SMBTransactionFile),
    TREECONNECT(SMBTransactionTreeConnect),
    NEGOTIATE(SMBTransactionNegotiate),
    DCERPC(SMBTransactionDCERPC),
    CREATE(SMBTransactionCreate),
    SESSIONSETUP(SMBTransactionSessionSetup),
    IOCTL(SMBTransactionIoctl),
    RENAME(SMBTransactionRename),
    SETFILEPATHINFO(SMBTransactionSetFilePathInfo),
}

// Used for Trans2 SET_PATH_INFO and SET_FILE_INFO
#[derive(Debug)]
pub struct SMBTransactionSetFilePathInfo {
    pub subcmd: u16,
    pub loi: u16,
    pub delete_on_close: bool,
    pub filename: Vec<u8>,
    pub fid: Vec<u8>,
}

impl SMBTransactionSetFilePathInfo {
    pub fn new(filename: Vec<u8>, fid: Vec<u8>, subcmd: u16, loi: u16, delete_on_close: bool)
        -> Self
    {
        return Self {
            filename: filename, fid: fid,
            subcmd: subcmd,
            loi: loi,
            delete_on_close: delete_on_close,
        }
    }
}

impl SMBState {
    pub fn new_setfileinfo_tx(&mut self, filename: Vec<u8>, fid: Vec<u8>,
            subcmd: u16, loi: u16, delete_on_close: bool)
        -> &mut SMBTransaction
    {
        let mut tx = self.new_tx();

        tx.type_data = Some(SMBTransactionTypeData::SETFILEPATHINFO(
                    SMBTransactionSetFilePathInfo::new(
                        filename, fid, subcmd, loi, delete_on_close)));
        tx.request_done = true;
        tx.response_done = self.tc_trunc; // no response expected if tc is truncated

        SCLogDebug!("SMB: TX SETFILEPATHINFO created: ID {}", tx.id);
        self.transactions.push(tx);
        let tx_ref = self.transactions.last_mut();
        return tx_ref.unwrap();
    }

    pub fn new_setpathinfo_tx(&mut self, filename: Vec<u8>,
            subcmd: u16, loi: u16, delete_on_close: bool)
        -> &mut SMBTransaction
    {
        let mut tx = self.new_tx();

        let fid : Vec<u8> = Vec::new();
        tx.type_data = Some(SMBTransactionTypeData::SETFILEPATHINFO(
                    SMBTransactionSetFilePathInfo::new(filename, fid,
                        subcmd, loi, delete_on_close)));
        tx.request_done = true;
        tx.response_done = self.tc_trunc; // no response expected if tc is truncated

        SCLogDebug!("SMB: TX SETFILEPATHINFO created: ID {}", tx.id);
        self.transactions.push(tx);
        let tx_ref = self.transactions.last_mut();
        return tx_ref.unwrap();
    }
}

#[derive(Debug)]
pub struct SMBTransactionRename {
    pub oldname: Vec<u8>,
    pub newname: Vec<u8>,
    pub fuid: Vec<u8>,
}

impl SMBTransactionRename {
    pub fn new(fuid: Vec<u8>, oldname: Vec<u8>, newname: Vec<u8>) -> Self {
        return Self {
            fuid: fuid, oldname: oldname, newname: newname,
        }
    }
}

impl SMBState {
    pub fn new_rename_tx(&mut self, fuid: Vec<u8>, oldname: Vec<u8>, newname: Vec<u8>)
        -> &mut SMBTransaction
    {
        let mut tx = self.new_tx();

        tx.type_data = Some(SMBTransactionTypeData::RENAME(
                    SMBTransactionRename::new(fuid, oldname, newname)));
        tx.request_done = true;
        tx.response_done = self.tc_trunc; // no response expected if tc is truncated

        SCLogDebug!("SMB: TX RENAME created: ID {}", tx.id);
        self.transactions.push(tx);
        let tx_ref = self.transactions.last_mut();
        return tx_ref.unwrap();
    }
}

#[derive(Default, Debug)]
pub struct SMBTransactionCreate {
    pub disposition: u32,
    pub delete_on_close: bool,
    pub directory: bool,
    pub filename: Vec<u8>,
    pub guid: Vec<u8>,

    pub create_ts: u32,
    pub last_access_ts: u32,
    pub last_write_ts: u32,
    pub last_change_ts: u32,

    pub size: u64,
}

impl SMBTransactionCreate {
    pub fn new(filename: Vec<u8>, disp: u32, del: bool, dir: bool) -> Self {
        return Self {
            disposition: disp,
            delete_on_close: del,
            directory: dir,
            filename: filename,
            ..Default::default()
        }
    }
}

#[derive(Default, Debug)]
pub struct SMBTransactionNegotiate {
    pub smb_ver: u8,
    pub dialects: Vec<Vec<u8>>,
    pub dialects2: Vec<Vec<u8>>,

    // SMB1 doesn't have the client GUID
    pub client_guid: Option<Vec<u8>>,
    pub server_guid: Vec<u8>,
}

impl SMBTransactionNegotiate {
    pub fn new(smb_ver: u8) -> Self {
        return Self {
            smb_ver: smb_ver,
            server_guid: Vec::with_capacity(16),
            ..Default::default()
        }
    }
}

#[derive(Default, Debug)]
pub struct SMBTransactionTreeConnect {
    pub is_pipe: bool,
    pub share_type: u8,
    pub tree_id: u32,
    pub share_name: Vec<u8>,

    /// SMB1 service strings
    pub req_service: Option<Vec<u8>>,
    pub res_service: Option<Vec<u8>>,
}

impl SMBTransactionTreeConnect {
    pub fn new(share_name: Vec<u8>) -> Self {
        return Self {
            share_name:share_name,
            ..Default::default()
        }
    }
}

#[derive(Debug)]
pub struct SMBTransaction {
    pub id: u64,    /// internal id

    /// version, command and status
    pub vercmd: SMBVerCmdStat,
    /// session id, tree id, etc.
    pub hdr: SMBCommonHdr,

    /// for state tracking. false means this side is in progress, true
    /// that it's complete.
    pub request_done: bool,
    pub response_done: bool,

    /// Command specific data
    pub type_data: Option<SMBTransactionTypeData>,

    pub tx_data: AppLayerTxData,
}

impl Transaction for SMBTransaction {
    fn id(&self) -> u64 {
        self.id
    }
}

impl SMBTransaction {
    pub fn new() -> Self {
        return Self {
              id: 0,
              vercmd: SMBVerCmdStat::new(),
              hdr: SMBCommonHdr::init(),
              request_done: false,
              response_done: false,
              type_data: None,
              tx_data: AppLayerTxData::new(),
        }
    }

    pub fn set_status(&mut self, status: u32, is_dos_error: bool)
    {
        if is_dos_error {
            self.vercmd.set_status_dos_error(status);
        } else {
            self.vercmd.set_ntstatus(status);
        }
    }

    pub fn free(&mut self) {
        debug_validate_bug_on!(self.tx_data.files_opened > 1);
        debug_validate_bug_on!(self.tx_data.files_logged > 1);
    }
}

impl Drop for SMBTransaction {
    fn drop(&mut self) {
        self.free();
    }
}

#[derive(Hash, Eq, PartialEq, Debug, Clone)]
pub struct SMBFileGUIDOffset {
    pub guid: Vec<u8>,
    pub offset: u64,
}

impl SMBFileGUIDOffset {
    pub fn new(guid: Vec<u8>, offset: u64) -> Self {
        Self {
            guid:guid,
            offset:offset,
        }
    }
}

/// type values to make sure we're not mixing things
/// up in hashmap lookups
pub const SMBHDR_TYPE_GUID:        u32 = 1;
pub const SMBHDR_TYPE_SHARE:       u32 = 2;
pub const SMBHDR_TYPE_FILENAME:    u32 = 3;
pub const SMBHDR_TYPE_OFFSET:      u32 = 4;
pub const SMBHDR_TYPE_GENERICTX:   u32 = 5;
pub const SMBHDR_TYPE_HEADER:      u32 = 6;
pub const SMBHDR_TYPE_MAX_SIZE:    u32 = 7; // max resp size for SMB1_COMMAND_TRANS
pub const SMBHDR_TYPE_TRANS_FRAG:  u32 = 8;
pub const SMBHDR_TYPE_TREE:        u32 = 9;
pub const SMBHDR_TYPE_DCERPCTX:    u32 = 10;

#[derive(Default, Hash, Eq, PartialEq, Debug)]
pub struct SMBCommonHdr {
    pub ssn_id: u64,
    pub tree_id: u32,
    pub rec_type: u32,
    pub msg_id: u64,
}

impl SMBCommonHdr {
    pub fn init() -> Self {
        Default::default()
    }
    pub fn new(rec_type: u32, ssn_id: u64, tree_id: u32, msg_id: u64) -> Self {
        Self {
            rec_type : rec_type,
            ssn_id : ssn_id,
            tree_id : tree_id,
            msg_id : msg_id,
        }
    }
    pub fn from2(r: &Smb2Record, rec_type: u32) -> SMBCommonHdr {
        let tree_id = match rec_type {
            SMBHDR_TYPE_TREE => { 0 },
            _ => r.tree_id,
        };
        let msg_id = match rec_type {
            SMBHDR_TYPE_TRANS_FRAG | SMBHDR_TYPE_SHARE => { 0 },
            _ => { r.message_id as u64 },
        };

        SMBCommonHdr {
            rec_type : rec_type,
            ssn_id : r.session_id,
            tree_id : tree_id,
            msg_id : msg_id,
        }

    }
    pub fn from1(r: &SmbRecord, rec_type: u32) -> SMBCommonHdr {
        let tree_id = match rec_type {
            SMBHDR_TYPE_TREE => { 0 },
            _ => r.tree_id as u32,
        };
        let msg_id = match rec_type {
            SMBHDR_TYPE_TRANS_FRAG | SMBHDR_TYPE_SHARE => { 0 },
            _ => { r.multiplex_id as u64 },
        };

        SMBCommonHdr {
            rec_type : rec_type,
            ssn_id : r.ssn_id as u64,
            tree_id : tree_id,
            msg_id : msg_id,
        }
    }

    // don't include tree id
    pub fn compare(&self, hdr: &SMBCommonHdr) -> bool {
        self.rec_type == hdr.rec_type && self.ssn_id == hdr.ssn_id &&
            self.msg_id == hdr.msg_id
    }
}

#[derive(Hash, Eq, PartialEq, Debug)]
pub struct SMBHashKeyHdrGuid {
    hdr: SMBCommonHdr,
    guid: Vec<u8>,
}

impl SMBHashKeyHdrGuid {
    pub fn new(hdr: SMBCommonHdr, guid: Vec<u8>) -> Self {
        Self {
            hdr: hdr, guid: guid,
        }
    }
}

#[derive(Hash, Eq, PartialEq, Debug)]
pub struct SMBTree {
    pub name: Vec<u8>,
    pub is_pipe: bool,
}

impl SMBTree {
    pub fn new(name: Vec<u8>, is_pipe: bool) -> Self {
        Self {
            name:name,
            is_pipe:is_pipe,
        }
    }
}

pub fn u32_as_bytes(i: u32) -> [u8;4] {
    let o1: u8 = ((i >> 24) & 0xff) as u8;
    let o2: u8 = ((i >> 16) & 0xff) as u8;
    let o3: u8 = ((i >> 8)  & 0xff) as u8;
    let o4: u8 =  (i        & 0xff) as u8;
    return [o1, o2, o3, o4]
}

#[derive(Default, Debug)]
pub struct SMBState<> {
    /// map ssn/tree/msgid to vec (guid/name/share)
    pub ssn2vec_map: HashMap<SMBCommonHdr, Vec<u8>>,
    /// map guid to filename
    pub guid2name_map: HashMap<Vec<u8>, Vec<u8>>,
    /// map ssn key to read offset
    pub ssn2vecoffset_map: HashMap<SMBCommonHdr, SMBFileGUIDOffset>,

    pub ssn2tree_map: HashMap<SMBCommonHdr, SMBTree>,

    // store partial data records that are transfered in multiple
    // requests for DCERPC.
    pub ssnguid2vec_map: HashMap<SMBHashKeyHdrGuid, Vec<u8>>,

    pub files: Files,

    skip_ts: u32,
    skip_tc: u32,

    pub file_ts_left : u32,
    pub file_tc_left : u32,
    pub file_ts_guid : Vec<u8>,
    pub file_tc_guid : Vec<u8>,

    pub ts_ssn_gap: bool,
    pub tc_ssn_gap: bool,

    pub ts_gap: bool, // last TS update was gap
    pub tc_gap: bool, // last TC update was gap

    pub ts_trunc: bool, // no more data for TOSERVER
    pub tc_trunc: bool, // no more data for TOCLIENT

    /// true as long as we have file txs that are in a post-gap
    /// state. It means we'll do extra house keeping for those.
    check_post_gap_file_txs: bool,
    post_gap_files_checked: bool,

    /// transactions list
    pub transactions: Vec<SMBTransaction>,

    /// tx counter for assigning incrementing id's to tx's
    tx_id: u64,

    /// SMB2 dialect or 0 if not set or SMB1
    pub dialect: u16,
    /// contains name of SMB1 dialect
    pub dialect_vec: Option<Vec<u8>>, // used if dialect == 0

    /// dcerpc interfaces, stored here to be able to match
    /// them while inspecting DCERPC REQUEST txs
    pub dcerpc_ifaces: Option<Vec<DCERPCIface>>,

    pub max_read_size: u32,
    pub max_write_size: u32,

    /// Timestamp in seconds of last update. This is packet time,
    /// potentially coming from pcaps.
    ts: u64,
}

impl State<SMBTransaction> for SMBState {
    fn get_transaction_count(&self) -> usize {
        self.transactions.len()
    }

    fn get_transaction_by_index(&self, index: usize) -> Option<&SMBTransaction> {
        self.transactions.get(index)
    }
}

impl SMBState {
    /// Allocation function for a new TLS parser instance
    pub fn new() -> Self {
        Self {
            ssn2vec_map:HashMap::new(),
            guid2name_map:HashMap::new(),
            ssn2vecoffset_map:HashMap::new(),
            ssn2tree_map:HashMap::new(),
            ssnguid2vec_map:HashMap::new(),
            files: Files::default(),
            skip_ts:0,
            skip_tc:0,
            file_ts_left:0,
            file_tc_left:0,
            file_ts_guid:Vec::new(),
            file_tc_guid:Vec::new(),
            ts_ssn_gap: false,
            tc_ssn_gap: false,
            ts_gap: false,
            tc_gap: false,
            ts_trunc: false,
            tc_trunc: false,
            check_post_gap_file_txs: false,
            post_gap_files_checked: false,
            transactions: Vec::new(),
            tx_id:0,
            dialect:0,
            dialect_vec: None,
            dcerpc_ifaces: None,
            ts: 0,
            ..Default::default()
        }
    }

    pub fn free(&mut self) {
        //self._debug_state_stats();
        self._debug_tx_stats();
    }

    pub fn new_tx(&mut self) -> SMBTransaction {
        let mut tx = SMBTransaction::new();
        self.tx_id += 1;
        tx.id = self.tx_id;
        SCLogDebug!("TX {} created", tx.id);
        return tx;
    }

    pub fn free_tx(&mut self, tx_id: u64) {
        SCLogDebug!("Freeing TX with ID {} TX.ID {}", tx_id, tx_id+1);
        let len = self.transactions.len();
        let mut found = false;
        let mut index = 0;
        for i in 0..len {
            let tx = &self.transactions[i];
            if tx.id == tx_id + 1 {
                found = true;
                index = i;
                SCLogDebug!("tx {} progress {}/{}", tx.id, tx.request_done, tx.response_done);
                break;
            }
        }
        if found {
            SCLogDebug!("freeing TX with ID {} TX.ID {} at index {} left: {} max id: {}",
                    tx_id, tx_id+1, index, self.transactions.len(), self.tx_id);
            self.transactions.remove(index);
        }
    }

    pub fn get_tx_by_id(&mut self, tx_id: u64) -> Option<&SMBTransaction> {
/*
        if self.transactions.len() > 100 {
            SCLogNotice!("get_tx_by_id: tx_id={} in list={}", tx_id, self.transactions.len());
            self._dump_txs();
            panic!("txs exploded");
        }
*/
        for tx in &mut self.transactions {
            if tx.id == tx_id + 1 {
                let ver = tx.vercmd.get_version();
                let mut _smbcmd;
                if ver == 2 {
                    let (_, cmd) = tx.vercmd.get_smb2_cmd();
                    _smbcmd = cmd;
                } else {
                    let (_, cmd) = tx.vercmd.get_smb1_cmd();
                    _smbcmd = cmd as u16;
                }
                SCLogDebug!("Found SMB TX: id {} ver:{} cmd:{} progress {}/{} type_data {:?}",
                        tx.id, ver, _smbcmd, tx.request_done, tx.response_done, tx.type_data);
                return Some(tx);
            }
        }
        SCLogDebug!("Failed to find SMB TX with ID {}", tx_id);
        return None;
    }

    fn update_ts(&mut self, ts: u64) {
        if ts != self.ts {
            self.ts = ts;
            self.post_gap_files_checked = false;
        }
    }

    /* generic TX has no type_data and is only used to
     * track a single cmd request/reply pair. */

    pub fn new_generic_tx(&mut self, smb_ver: u8, smb_cmd: u16, key: SMBCommonHdr)
        -> &mut SMBTransaction
    {
        let mut tx = self.new_tx();
        if smb_ver == 1 && smb_cmd <= 255 {
            tx.vercmd.set_smb1_cmd(smb_cmd as u8);
        } else if smb_ver == 2 {
            tx.vercmd.set_smb2_cmd(smb_cmd);
        }

        tx.type_data = None;
        tx.request_done = true;
        tx.response_done = self.tc_trunc; // no response expected if tc is truncated
        tx.hdr = key;

        SCLogDebug!("SMB: TX GENERIC created: ID {} tx list {} {:?}",
                tx.id, self.transactions.len(), &tx);
        self.transactions.push(tx);
        let tx_ref = self.transactions.last_mut();
        return tx_ref.unwrap();
    }

    pub fn get_last_tx(&mut self, smb_ver: u8, smb_cmd: u16)
        -> Option<&mut SMBTransaction>
    {
        let tx_ref = self.transactions.last_mut();
        match tx_ref {
            Some(tx) => {
                let found = if tx.vercmd.get_version() == smb_ver {
                    if smb_ver == 1 {
                        let (_, cmd) = tx.vercmd.get_smb1_cmd();
                        cmd as u16 == smb_cmd
                    } else if smb_ver == 2 {
                        let (_, cmd) = tx.vercmd.get_smb2_cmd();
                        cmd == smb_cmd
                    } else {
                        false
                    }
                } else {
                    false
                };
                if found {
                    return Some(tx);
                }
            },
            None => { },
        }
        return None;
    }

    pub fn get_generic_tx(&mut self, smb_ver: u8, smb_cmd: u16, key: &SMBCommonHdr)
        -> Option<&mut SMBTransaction>
    {
        for tx in &mut self.transactions {
            let found = if tx.vercmd.get_version() == smb_ver {
                if smb_ver == 1 {
                    let (_, cmd) = tx.vercmd.get_smb1_cmd();
                    cmd as u16 == smb_cmd && tx.hdr.compare(key)
                } else if smb_ver == 2 {
                    let (_, cmd) = tx.vercmd.get_smb2_cmd();
                    cmd == smb_cmd && tx.hdr.compare(key)
                } else {
                    false
                }
            } else {
                false
            };
            if found {
                return Some(tx);
            }
        }
        return None;
    }

    pub fn new_negotiate_tx(&mut self, smb_ver: u8)
        -> &mut SMBTransaction
    {
        let mut tx = self.new_tx();
        if smb_ver == 1 {
            tx.vercmd.set_smb1_cmd(SMB1_COMMAND_NEGOTIATE_PROTOCOL);
        } else if smb_ver == 2 {
            tx.vercmd.set_smb2_cmd(SMB2_COMMAND_NEGOTIATE_PROTOCOL);
        }

        tx.type_data = Some(SMBTransactionTypeData::NEGOTIATE(
                    SMBTransactionNegotiate::new(smb_ver)));
        tx.request_done = true;
        tx.response_done = self.tc_trunc; // no response expected if tc is truncated

        SCLogDebug!("SMB: TX NEGOTIATE created: ID {} SMB ver {}", tx.id, smb_ver);
        self.transactions.push(tx);
        let tx_ref = self.transactions.last_mut();
        return tx_ref.unwrap();
    }

    pub fn get_negotiate_tx(&mut self, smb_ver: u8)
        -> Option<&mut SMBTransaction>
    {
        for tx in &mut self.transactions {
            let found = match tx.type_data {
                Some(SMBTransactionTypeData::NEGOTIATE(ref x)) => {
                    if x.smb_ver == smb_ver {
                        true
                    } else {
                        false
                    }
                },
                _ => { false },
            };
            if found {
                return Some(tx);
            }
        }
        return None;
    }

    pub fn new_treeconnect_tx(&mut self, hdr: SMBCommonHdr, name: Vec<u8>)
        -> &mut SMBTransaction
    {
        let mut tx = self.new_tx();

        tx.hdr = hdr;
        tx.type_data = Some(SMBTransactionTypeData::TREECONNECT(
                    SMBTransactionTreeConnect::new(name.to_vec())));
        tx.request_done = true;
        tx.response_done = self.tc_trunc; // no response expected if tc is truncated

        SCLogDebug!("SMB: TX TREECONNECT created: ID {} NAME {}",
                tx.id, String::from_utf8_lossy(&name));
        self.transactions.push(tx);
        let tx_ref = self.transactions.last_mut();
        return tx_ref.unwrap();
    }

    pub fn get_treeconnect_tx(&mut self, hdr: SMBCommonHdr)
        -> Option<&mut SMBTransaction>
    {
        for tx in &mut self.transactions {
            let hit = tx.hdr.compare(&hdr) && match tx.type_data {
                Some(SMBTransactionTypeData::TREECONNECT(_)) => { true },
                _ => { false },
            };
            if hit {
                return Some(tx);
            }
        }
        return None;
    }

    pub fn new_create_tx(&mut self, file_name: &Vec<u8>,
            disposition: u32, del: bool, dir: bool,
            hdr: SMBCommonHdr)
        -> &mut SMBTransaction
    {
        let mut tx = self.new_tx();
        tx.hdr = hdr;
        tx.type_data = Some(SMBTransactionTypeData::CREATE(
                            SMBTransactionCreate::new(
                                file_name.to_vec(), disposition,
                                del, dir)));
        tx.request_done = true;
        tx.response_done = self.tc_trunc; // no response expected if tc is truncated

        self.transactions.push(tx);
        let tx_ref = self.transactions.last_mut();
        return tx_ref.unwrap();
    }

    pub fn get_create_tx_by_hdr(&mut self, hdr: &SMBCommonHdr)
        -> Option<&mut SMBTransaction>
    {
        for tx in &mut self.transactions {
            let found = match tx.type_data {
                Some(SMBTransactionTypeData::CREATE(ref _d)) => {
                    tx.hdr.compare(hdr)
                },
                _ => { false },
            };

            if found {
                SCLogDebug!("SMB: Found SMB create TX with ID {}", tx.id);
                return Some(tx);
            }
        }
        SCLogDebug!("SMB: Failed to find SMB create TX with key {:?}", hdr);
        return None;
    }

    pub fn get_service_for_guid(&self, guid: &[u8]) -> (&'static str, bool)
    {
        let (name, is_dcerpc) = match self.guid2name_map.get(&guid.to_vec()) {
            Some(n) => {
                let mut s = n.as_slice();
                // skip leading \ if we have it
                if s.len() > 1 && s[0] == 0x5c_u8 {
                    s = &s[1..];
                }
                match str::from_utf8(s) {
                    Ok("PSEXESVC") => ("PSEXESVC", false),
                    Ok("svcctl") => ("svcctl", true),
                    Ok("srvsvc") => ("srvsvc", true),
                    Ok("atsvc") => ("atsvc", true),
                    Ok("lsarpc") => ("lsarpc", true),
                    Ok("samr") => ("samr", true),
                    Ok("spoolss") => ("spoolss", true),
                    Ok("winreg") => ("winreg", true),
                    Ok("suricata::dcerpc") => ("unknown", true),
                    Err(_) => ("MALFORMED", false),
                    Ok(&_) => {
                        SCLogDebug!("don't know {}", String::from_utf8_lossy(&n));
                        ("UNKNOWN", false)
                    },
                }
            },
            _ => { ("UNKNOWN", false) },
        };
        SCLogDebug!("service {} is_dcerpc {}", name, is_dcerpc);
        (name, is_dcerpc)
    }

    fn post_gap_housekeeping_for_files(&mut self)
    {
        let mut post_gap_txs = false;
        for tx in &mut self.transactions {
            if let Some(SMBTransactionTypeData::FILE(ref mut f)) = tx.type_data {
                if f.post_gap_ts > 0 {
                    if self.ts > f.post_gap_ts {
                        tx.request_done = true;
                        tx.response_done = true;
                        let (files, flags) = self.files.get(f.direction);
                        f.file_tracker.trunc(files, flags);
                    } else {
                        post_gap_txs = true;
                    }
                }
            }
        }
        self.check_post_gap_file_txs = post_gap_txs;
    }

    /* after a gap we will consider all transactions complete for our
     * direction. File transfer transactions are an exception. Those
     * can handle gaps. For the file transactions we set the current
     * (flow) time and prune them in 60 seconds if no update for them
     * was received. */
    fn post_gap_housekeeping(&mut self, dir: Direction)
    {
        if self.ts_ssn_gap && dir == Direction::ToServer {
            for tx in &mut self.transactions {
                if tx.id >= self.tx_id {
                    SCLogDebug!("post_gap_housekeeping: done");
                    break;
                }
                if let Some(SMBTransactionTypeData::FILE(ref mut f)) = tx.type_data {
                    // leaving FILE txs open as they can deal with gaps. We
                    // remove them after 60 seconds of no activity though.
                    if f.post_gap_ts == 0 {
                        f.post_gap_ts = self.ts + 60;
                        self.check_post_gap_file_txs = true;
                    }
                } else {
                    SCLogDebug!("post_gap_housekeeping: tx {} marked as done TS", tx.id);
                    tx.request_done = true;
                }
            }
        } else if self.tc_ssn_gap && dir == Direction::ToClient {
            for tx in &mut self.transactions {
                if tx.id >= self.tx_id {
                    SCLogDebug!("post_gap_housekeeping: done");
                    break;
                }
                if let Some(SMBTransactionTypeData::FILE(ref mut f)) = tx.type_data {
                    // leaving FILE txs open as they can deal with gaps. We
                    // remove them after 60 seconds of no activity though.
                    if f.post_gap_ts == 0 {
                        f.post_gap_ts = self.ts + 60;
                        self.check_post_gap_file_txs = true;
                    }
                } else {
                    SCLogDebug!("post_gap_housekeeping: tx {} marked as done TC", tx.id);
                    tx.request_done = true;
                    tx.response_done = true;
                }
            }

        }
    }

    pub fn set_file_left(&mut self, direction: Direction, rec_size: u32, data_size: u32, fuid: Vec<u8>)
    {
        let left = rec_size.saturating_sub(data_size);
        if direction == Direction::ToServer {
            self.file_ts_left = left;
            self.file_ts_guid = fuid;
        } else {
            self.file_tc_left = left;
            self.file_tc_guid = fuid;
        }
    }

    pub fn set_skip(&mut self, direction: Direction, rec_size: u32, data_size: u32)
    {
        let skip = rec_size.saturating_sub(data_size);
        if direction == Direction::ToServer {
            self.skip_ts = skip;
        } else {
            self.skip_tc = skip;
        }
    }

    // return how much data we consumed
    fn handle_skip(&mut self, direction: Direction, input_size: u32) -> u32 {
        let mut skip_left = if direction == Direction::ToServer {
            self.skip_ts
        } else {
            self.skip_tc
        };
        if skip_left == 0 {
            return 0
        }
        SCLogDebug!("skip_left {} input_size {}", skip_left, input_size);

        let consumed = if skip_left >= input_size {
            input_size
        } else {
            skip_left
        };

        if skip_left <= input_size {
            skip_left = 0;
        } else {
            skip_left -= input_size;
        }

        if direction == Direction::ToServer {
            self.skip_ts = skip_left;
        } else {
            self.skip_tc = skip_left;
        }
        return consumed;
    }

    fn add_nbss_ts_frames(&mut self, flow: *const Flow, stream_slice: &StreamSlice, input: &[u8], nbss_len: i64) -> (Option<Frame>, Option<Frame>, Option<Frame>) {
        let nbss_pdu = Frame::new_ts(flow, stream_slice, input, nbss_len + 4, SMBFrameType::NBSSPdu as u8);
        SCLogDebug!("NBSS PDU frame {:?}", nbss_pdu);
        let nbss_hdr_frame = Frame::new_ts(flow, stream_slice, input, 4 as i64, SMBFrameType::NBSSHdr as u8);
        SCLogDebug!("NBSS HDR frame {:?}", nbss_hdr_frame);
        let nbss_data_frame = Frame::new_ts(flow, stream_slice, &input[4..], nbss_len, SMBFrameType::NBSSData as u8);
        SCLogDebug!("NBSS DATA frame {:?}", nbss_data_frame);
        (nbss_pdu, nbss_hdr_frame, nbss_data_frame)
    }

    fn add_smb1_ts_pdu_frame(&mut self, flow: *const Flow, stream_slice: &StreamSlice, input: &[u8], nbss_len: i64) -> Option<Frame> {
        let smb_pdu = Frame::new_ts(flow, stream_slice, input, nbss_len, SMBFrameType::SMB1Pdu as u8);
        SCLogDebug!("SMB PDU frame {:?}", smb_pdu);
        smb_pdu
    }
    fn add_smb1_ts_hdr_data_frames(&mut self, flow: *const Flow, stream_slice: &StreamSlice, input: &[u8], nbss_len: i64) {
        let _smb1_hdr = Frame::new_ts(flow, stream_slice, input, 32 as i64, SMBFrameType::SMB1Hdr as u8);
        SCLogDebug!("SMBv1 HDR frame {:?}", _smb1_hdr);
        if input.len() > 32 {
            let _smb1_data = Frame::new_ts(flow, stream_slice, &input[32..], (nbss_len - 32) as i64, SMBFrameType::SMB1Data as u8);
            SCLogDebug!("SMBv1 DATA frame {:?}", _smb1_data);
        }
    }

    fn add_smb2_ts_pdu_frame(&mut self, flow: *const Flow, stream_slice: &StreamSlice, input: &[u8], nbss_len: i64) -> Option<Frame> {
        let smb_pdu = Frame::new_ts(flow, stream_slice, input, nbss_len, SMBFrameType::SMB2Pdu as u8);
        SCLogDebug!("SMBv2 PDU frame {:?}", smb_pdu);
        smb_pdu
    }
    fn add_smb2_ts_hdr_data_frames(&mut self, flow: *const Flow, stream_slice: &StreamSlice, input: &[u8], nbss_len: i64, hdr_len: i64) {
        let _smb2_hdr = Frame::new_ts(flow, stream_slice, input, hdr_len, SMBFrameType::SMB2Hdr as u8);
        SCLogDebug!("SMBv2 HDR frame {:?}", _smb2_hdr);
        if input.len() > hdr_len as usize {
            let _smb2_data = Frame::new_ts(flow, stream_slice, &input[hdr_len as usize..], nbss_len - hdr_len, SMBFrameType::SMB2Data as u8);
            SCLogDebug!("SMBv2 DATA frame {:?}", _smb2_data);
        }
    }

    fn add_smb3_ts_pdu_frame(&mut self, flow: *const Flow, stream_slice: &StreamSlice, input: &[u8], nbss_len: i64) -> Option<Frame> {
        let smb_pdu = Frame::new_ts(flow, stream_slice, input, nbss_len, SMBFrameType::SMB3Pdu as u8);
        SCLogDebug!("SMBv3 PDU frame {:?}", smb_pdu);
        smb_pdu
    }
    fn add_smb3_ts_hdr_data_frames(&mut self, flow: *const Flow, stream_slice: &StreamSlice, input: &[u8], nbss_len: i64) {
        let _smb3_hdr = Frame::new_ts(flow, stream_slice, input, 52 as i64, SMBFrameType::SMB3Hdr as u8);
        SCLogDebug!("SMBv3 HDR frame {:?}", _smb3_hdr);
        if input.len() > 52 {
            let _smb3_data = Frame::new_ts(flow, stream_slice, &input[52..], (nbss_len - 52) as i64, SMBFrameType::SMB3Data as u8);
            SCLogDebug!("SMBv3 DATA frame {:?}", _smb3_data);
        }
    }

    /// return bytes consumed
    pub fn parse_tcp_data_ts_partial<'b>(&mut self, flow: *const Flow, stream_slice: &StreamSlice, input: &'b[u8]) -> usize
    {
        SCLogDebug!("incomplete of size {}", input.len());
        if input.len() < 512 {
            // check for malformed data. Wireshark reports as
            // 'NBSS continuation data'. If it's invalid we're
            // lost so we give up.
            if input.len() > 8 {
                match parse_nbss_record_partial(input) {
                    Ok((_, ref hdr)) => {
                        if !hdr.is_smb() {
                            SCLogDebug!("partial NBSS, not SMB and no known msg type {}", hdr.message_type);
                            self.trunc_ts();
                            return 0;
                        }
                    },
                    _ => {},
                }
            }
            return 0;
        }

        match parse_nbss_record_partial(input) {
            Ok((output, ref nbss_part_hdr)) => {
                SCLogDebug!("parse_nbss_record_partial ok, output len {}", output.len());
                if nbss_part_hdr.message_type == NBSS_MSGTYPE_SESSION_MESSAGE {
                    match parse_smb_version(nbss_part_hdr.data) {
                        Ok((_, ref smb)) => {
                            SCLogDebug!("SMB {:?}", smb);
                            if smb.version == 0xff_u8 { // SMB1
                                SCLogDebug!("SMBv1 record");
                                match parse_smb_record(nbss_part_hdr.data) {
                                    Ok((_, ref r)) => {
                                        if r.command == SMB1_COMMAND_WRITE_ANDX {
                                            // see if it's a write to a pipe. We only handle those
                                            // if complete.
                                            let tree_key = SMBCommonHdr::new(SMBHDR_TYPE_SHARE,
                                                    r.ssn_id as u64, r.tree_id as u32, 0);
                                            let is_pipe = match self.ssn2tree_map.get(&tree_key) {
                                                Some(n) => n.is_pipe,
                                                None => false,
                                            };
                                            if is_pipe {
                                                return 0;
                                            }
                                            smb1_write_request_record(self, r, SMB1_HEADER_SIZE, SMB1_COMMAND_WRITE_ANDX);

                                            self.add_nbss_ts_frames(flow, stream_slice, input, nbss_part_hdr.length as i64);
                                            self.add_smb1_ts_pdu_frame(flow, stream_slice, nbss_part_hdr.data, nbss_part_hdr.length as i64);
                                            self.add_smb1_ts_hdr_data_frames(flow, stream_slice, nbss_part_hdr.data, nbss_part_hdr.length as i64);

                                            let consumed = input.len() - output.len();
                                            return consumed;
                                        }
                                    },
                                    _ => { },

                                }
                            } else if smb.version == 0xfe_u8 { // SMB2
                                SCLogDebug!("SMBv2 record");
                                match parse_smb2_request_record(nbss_part_hdr.data) {
                                    Ok((_, ref smb_record)) => {
                                        SCLogDebug!("SMB2: partial record {}",
                                                &smb2_command_string(smb_record.command));
                                        if smb_record.command == SMB2_COMMAND_WRITE {
                                            smb2_write_request_record(self, smb_record);

                                            self.add_nbss_ts_frames(flow, stream_slice, input, nbss_part_hdr.length as i64);
                                            self.add_smb2_ts_pdu_frame(flow, stream_slice, nbss_part_hdr.data, nbss_part_hdr.length as i64);
                                            self.add_smb2_ts_hdr_data_frames(flow, stream_slice, nbss_part_hdr.data, nbss_part_hdr.length as i64, smb_record.header_len as i64);

                                            let consumed = input.len() - output.len();
                                            SCLogDebug!("consumed {}", consumed);
                                            return consumed;
                                        }
                                    },
                                    _ => { },
                                }
                            }
                            // no SMB3 here yet, will buffer full records
                        },
                        _ => { },
                    }
                }
            },
            _ => { },
        }

        return 0;
    }

    /// Parsing function, handling TCP chunks fragmentation
    pub fn parse_tcp_data_ts<'b>(&mut self, flow: *const Flow, stream_slice: &StreamSlice) -> AppLayerResult
    {
        let mut cur_i = stream_slice.as_slice();
        let consumed = self.handle_skip(Direction::ToServer, cur_i.len() as u32);
        if consumed > 0 {
            if consumed > cur_i.len() as u32 {
                self.set_event(SMBEvent::InternalError);
                return AppLayerResult::err();
            }
            cur_i = &cur_i[consumed as usize..];
        }
        // take care of in progress file chunk transfers
        // and skip buffer beyond it
        let consumed = self.filetracker_update(Direction::ToServer, cur_i, 0);
        if consumed > 0 {
            if consumed > cur_i.len() as u32 {
                self.set_event(SMBEvent::InternalError);
                return AppLayerResult::err();
            }
            cur_i = &cur_i[consumed as usize..];
        }
        if cur_i.len() == 0 {
            return AppLayerResult::ok();
        }
        // gap
        if self.ts_gap {
            SCLogDebug!("TS trying to catch up after GAP (input {})", cur_i.len());
            while cur_i.len() > 0 { // min record size
                match search_smb_record(cur_i) {
                    Ok((_, pg)) => {
                        SCLogDebug!("smb record found");
                        let smb2_offset = cur_i.len() - pg.len();
                        if smb2_offset < 4 {
                            cur_i = &cur_i[smb2_offset+4..];
                            continue; // see if we have another record in our data
                        }
                        let nbss_offset = smb2_offset - 4;
                        cur_i = &cur_i[nbss_offset..];

                        self.ts_gap = false;
                        break;
                    },
                    _ => {
                        let mut consumed = stream_slice.len();
                        if consumed < 4 {
                            consumed = 0;
                        } else {
                            consumed = consumed - 3;
                        }
                        SCLogDebug!("smb record NOT found");
                        return AppLayerResult::incomplete(consumed as u32, 8);
                    },
                }
            }
        }
        while cur_i.len() > 0 { // min record size
            match parse_nbss_record(cur_i) {
                Ok((rem, ref nbss_hdr)) => {
                    SCLogDebug!("nbss frame offset {} len {}", stream_slice.offset_from(cur_i), cur_i.len() - rem.len());
                    let (_, _, nbss_data_frame) = self.add_nbss_ts_frames(flow, stream_slice, cur_i, nbss_hdr.length as i64);

                    if nbss_hdr.message_type == NBSS_MSGTYPE_SESSION_MESSAGE {
                        // we have the full records size worth of data,
                        // let's parse it
                        match parse_smb_version(nbss_hdr.data) {
                            Ok((_, ref smb)) => {

                                SCLogDebug!("SMB {:?}", smb);
                                if smb.version == 0xff_u8 { // SMB1

                                    SCLogDebug!("SMBv1 record");
                                    match parse_smb_record(nbss_hdr.data) {
                                        Ok((_, ref smb_record)) => {
                                            let pdu_frame = self.add_smb1_ts_pdu_frame(flow, stream_slice, nbss_hdr.data, nbss_hdr.length as i64);
                                            self.add_smb1_ts_hdr_data_frames(flow, stream_slice, nbss_hdr.data, nbss_hdr.length as i64);
                                            if smb_record.is_request() {
                                                smb1_request_record(self, smb_record);
                                            } else {
                                                // If we recevied a response when expecting a request, set an event
                                                // on the PDU frame instead of handling the response.
                                                SCLogDebug!("SMB1 reply seen from client to server");
                                                if let Some(frame) = pdu_frame {
                                                    frame.add_event(flow, 0, SMBEvent::ResponseToServer as u8);
                                                }
                                            }
                                        },
                                        _ => {
                                            if let Some(frame) = nbss_data_frame {
                                                frame.add_event(flow, 0, SMBEvent::MalformedData as u8);
                                            }
                                            self.set_event(SMBEvent::MalformedData);
                                            return AppLayerResult::err();
                                        },
                                    }
                                } else if smb.version == 0xfe_u8 { // SMB2
                                    let mut nbss_data = nbss_hdr.data;
                                    while nbss_data.len() > 0 {
                                        SCLogDebug!("SMBv2 record");
                                        match parse_smb2_request_record(nbss_data) {
                                            Ok((nbss_data_rem, ref smb_record)) => {
                                                let record_len = (nbss_data.len() - nbss_data_rem.len()) as i64;
                                                let pdu_frame = self.add_smb2_ts_pdu_frame(flow, stream_slice, nbss_data, record_len);
                                                self.add_smb2_ts_hdr_data_frames(flow, stream_slice, nbss_data, record_len, smb_record.header_len as i64);
                                                SCLogDebug!("nbss_data_rem {}", nbss_data_rem.len());
                                                if smb_record.is_request() {
                                                    smb2_request_record(self, smb_record);
                                                } else {
                                                    // If we recevied a response when expecting a request, set an event
                                                    // on the PDU frame instead of handling the response.
                                                    SCLogDebug!("SMB2 reply seen from client to server");
                                                    if let Some(frame) = pdu_frame {
                                                        frame.add_event(flow, 0, SMBEvent::ResponseToServer as u8);
                                                    }
                                                }
                                                nbss_data = nbss_data_rem;
                                            },
                                            _ => {
                                                if let Some(frame) = nbss_data_frame {
                                                    frame.add_event(flow, 0, SMBEvent::MalformedData as u8);
                                                }
                                                self.set_event(SMBEvent::MalformedData);
                                                return AppLayerResult::err();
                                            },
                                        }
                                    }
                                } else if smb.version == 0xfd_u8 { // SMB3 transform

                                    let mut nbss_data = nbss_hdr.data;
                                    while nbss_data.len() > 0 {
                                        SCLogDebug!("SMBv3 transform record");
                                        match parse_smb3_transform_record(nbss_data) {
                                            Ok((nbss_data_rem, ref _smb3_record)) => {
                                                let record_len = (nbss_data.len() - nbss_data_rem.len()) as i64;
                                                self.add_smb3_ts_pdu_frame(flow, stream_slice, nbss_data, record_len);
                                                self.add_smb3_ts_hdr_data_frames(flow, stream_slice, nbss_data, record_len);
                                                nbss_data = nbss_data_rem;
                                            },
                                            _ => {
                                                if let Some(frame) = nbss_data_frame {
                                                    frame.add_event(flow, 0, SMBEvent::MalformedData as u8);
                                                }
                                                self.set_event(SMBEvent::MalformedData);
                                                return AppLayerResult::err();
                                            },
                                        }
                                    }
                                }
                            },
                            _ => {
                                self.set_event(SMBEvent::MalformedData);
                                return AppLayerResult::err();
                            },
                        }
                    } else {
                        SCLogDebug!("NBSS message {:X}", nbss_hdr.message_type);
                    }
                    cur_i = rem;
                },
                Err(Err::Incomplete(needed)) => {
                    if let Needed::Size(n) = needed {
                        let n = usize::from(n) + cur_i.len();
                        // 512 is the minimum for parse_tcp_data_ts_partial
                        if n >= 512 && cur_i.len() < 512 {
                            let total_consumed = stream_slice.offset_from(cur_i);
                            return AppLayerResult::incomplete(total_consumed, 512);
                        }
                        let consumed = self.parse_tcp_data_ts_partial(flow, stream_slice, cur_i);
                        if consumed == 0 {
                            // if we consumed none we will buffer the entire record
                            let total_consumed = stream_slice.offset_from(cur_i);
                            SCLogDebug!("setting consumed {} need {} needed {:?} total input {}",
                                    total_consumed, n, needed, stream_slice.len());
                            let need = n;
                            return AppLayerResult::incomplete(total_consumed as u32, need as u32);
                        }
                        // tracking a write record, which we don't need to
                        // queue up at the stream level, but can feed to us
                        // in small chunks
                        return AppLayerResult::ok();
                    } else {
                        self.set_event(SMBEvent::InternalError);
                        return AppLayerResult::err();
                    }
                },
                Err(_) => {
                    self.set_event(SMBEvent::MalformedData);
                    return AppLayerResult::err();
                },
            }
        };

        self.post_gap_housekeeping(Direction::ToServer);
        if self.check_post_gap_file_txs && !self.post_gap_files_checked {
            self.post_gap_housekeeping_for_files();
            self.post_gap_files_checked = true;
        }
        AppLayerResult::ok()
    }

    fn add_nbss_tc_frames(&mut self, flow: *const Flow, stream_slice: &StreamSlice, input: &[u8], nbss_len: i64) -> (Option<Frame>, Option<Frame>, Option<Frame>) {
        let nbss_pdu = Frame::new_tc(flow, stream_slice, input, nbss_len + 4, SMBFrameType::NBSSPdu as u8);
        SCLogDebug!("NBSS PDU frame {:?}", nbss_pdu);
        let nbss_hdr_frame = Frame::new_tc(flow, stream_slice, input, 4 as i64, SMBFrameType::NBSSHdr as u8);
        SCLogDebug!("NBSS HDR frame {:?}", nbss_hdr_frame);
        let nbss_data_frame = Frame::new_tc(flow, stream_slice, &input[4..], nbss_len, SMBFrameType::NBSSData as u8);
        SCLogDebug!("NBSS DATA frame {:?}", nbss_data_frame);
        (nbss_pdu, nbss_hdr_frame, nbss_data_frame)
    }

    fn add_smb1_tc_pdu_frame(&mut self, flow: *const Flow, stream_slice: &StreamSlice, input: &[u8], nbss_len: i64) -> Option<Frame> {
        let smb_pdu = Frame::new_tc(flow, stream_slice, input, nbss_len, SMBFrameType::SMB1Pdu as u8);
        SCLogDebug!("SMB PDU frame {:?}", smb_pdu);
        smb_pdu
    }
    fn add_smb1_tc_hdr_data_frames(&mut self, flow: *const Flow, stream_slice: &StreamSlice, input: &[u8], nbss_len: i64) {
        let _smb1_hdr = Frame::new_tc(flow, stream_slice, input, SMB1_HEADER_SIZE as i64, SMBFrameType::SMB1Hdr as u8);
        SCLogDebug!("SMBv1 HDR frame {:?}", _smb1_hdr);
        if input.len() > SMB1_HEADER_SIZE {
            let _smb1_data = Frame::new_tc(flow, stream_slice, &input[SMB1_HEADER_SIZE..], (nbss_len - SMB1_HEADER_SIZE as i64) as i64,
                    SMBFrameType::SMB1Data as u8);
            SCLogDebug!("SMBv1 DATA frame {:?}", _smb1_data);
        }
    }

    fn add_smb2_tc_pdu_frame(&mut self, flow: *const Flow, stream_slice: &StreamSlice, input: &[u8], nbss_len: i64) -> Option<Frame> {
        let smb_pdu = Frame::new_tc(flow, stream_slice, input, nbss_len, SMBFrameType::SMB2Pdu as u8);
        SCLogDebug!("SMBv2 PDU frame {:?}", smb_pdu);
        smb_pdu
    }
    fn add_smb2_tc_hdr_data_frames(&mut self, flow: *const Flow, stream_slice: &StreamSlice, input: &[u8], nbss_len: i64, hdr_len: i64) {
        let _smb2_hdr = Frame::new_tc(flow, stream_slice, input, hdr_len, SMBFrameType::SMB2Hdr as u8);
        SCLogDebug!("SMBv2 HDR frame {:?}", _smb2_hdr);
        if input.len() > hdr_len as usize {
            let _smb2_data = Frame::new_tc(flow, stream_slice, &input[hdr_len as usize ..], nbss_len - hdr_len, SMBFrameType::SMB2Data as u8);
            SCLogDebug!("SMBv2 DATA frame {:?}", _smb2_data);
        }
    }

    fn add_smb3_tc_pdu_frame(&mut self, flow: *const Flow, stream_slice: &StreamSlice, input: &[u8], nbss_len: i64) {
        let _smb_pdu = Frame::new_tc(flow, stream_slice, input, nbss_len, SMBFrameType::SMB3Pdu as u8);
        SCLogDebug!("SMBv3 PDU frame {:?}", _smb_pdu);
    }
    fn add_smb3_tc_hdr_data_frames(&mut self, flow: *const Flow, stream_slice: &StreamSlice, input: &[u8], nbss_len: i64) {
        let _smb3_hdr = Frame::new_tc(flow, stream_slice, input, 52 as i64, SMBFrameType::SMB3Hdr as u8);
        SCLogDebug!("SMBv3 HDR frame {:?}", _smb3_hdr);
        if input.len() > 52 {
            let _smb3_data = Frame::new_tc(flow, stream_slice, &input[52..], (nbss_len - 52) as i64, SMBFrameType::SMB3Data as u8);
            SCLogDebug!("SMBv3 DATA frame {:?}", _smb3_data);
        }
    }

    /// return bytes consumed
    pub fn parse_tcp_data_tc_partial<'b>(&mut self, flow: *const Flow, stream_slice: &StreamSlice, input: &'b[u8]) -> usize
    {
        SCLogDebug!("incomplete of size {}", input.len());
        if input.len() < 512 {
            // check for malformed data. Wireshark reports as
            // 'NBSS continuation data'. If it's invalid we're
            // lost so we give up.
            if input.len() > 8 {
                match parse_nbss_record_partial(input) {
                    Ok((_, ref hdr)) => {
                        if !hdr.is_smb() {
                            SCLogDebug!("partial NBSS, not SMB and no known msg type {}", hdr.message_type);
                            self.trunc_tc();
                            return 0;
                        }
                    },
                    _ => {},
                }
            }
            return 0;
        }

        if let Ok((output, ref nbss_part_hdr)) = parse_nbss_record_partial(input) {
            SCLogDebug!("parse_nbss_record_partial ok, output len {}", output.len());
            if nbss_part_hdr.message_type == NBSS_MSGTYPE_SESSION_MESSAGE {
                if let Ok((_, ref smb)) = parse_smb_version(nbss_part_hdr.data) {
                    SCLogDebug!("SMB {:?}", smb);
                    if smb.version == 255u8 { // SMB1
                        SCLogDebug!("SMBv1 record");
                        if let Ok((_, ref r)) = parse_smb_record(nbss_part_hdr.data) {
                            SCLogDebug!("SMB1: partial record {}",
                                    r.command);
                            if r.command == SMB1_COMMAND_READ_ANDX {
                                let tree_key = SMBCommonHdr::new(SMBHDR_TYPE_SHARE,
                                        r.ssn_id as u64, r.tree_id as u32, 0);
                                let is_pipe = match self.ssn2tree_map.get(&tree_key) {
                                    Some(n) => n.is_pipe,
                                        None => false,
                                };
                                if is_pipe {
                                    return 0;
                                }

                                // create NBSS frames here so we don't get double frames
                                // when we don't consume the data now.
                                self.add_nbss_tc_frames(flow, stream_slice, input, nbss_part_hdr.length as i64);
                                self.add_smb1_tc_pdu_frame(flow, stream_slice, nbss_part_hdr.data, nbss_part_hdr.length as i64);
                                self.add_smb1_tc_hdr_data_frames(flow, stream_slice, nbss_part_hdr.data, nbss_part_hdr.length as i64);

                                smb1_read_response_record(self, r, SMB1_HEADER_SIZE);
                                let consumed = input.len() - output.len();
                                return consumed;
                            }
                        }
                    } else if smb.version == 254u8 { // SMB2
                        SCLogDebug!("SMBv2 record");
                        if let Ok((_, ref smb_record)) = parse_smb2_response_record(nbss_part_hdr.data) {
                            SCLogDebug!("SMB2: partial record {}",
                                    &smb2_command_string(smb_record.command));
                            if smb_record.command == SMB2_COMMAND_READ {
                                // create NBSS frames here so we don't get double frames
                                // when we don't consume the data now.
                                self.add_nbss_tc_frames(flow, stream_slice, input, nbss_part_hdr.length as i64);
                                self.add_smb2_tc_pdu_frame(flow, stream_slice, nbss_part_hdr.data, nbss_part_hdr.length as i64);
                                self.add_smb2_tc_hdr_data_frames(flow, stream_slice, nbss_part_hdr.data, nbss_part_hdr.length as i64, smb_record.header_len as i64);

                                smb2_read_response_record(self, smb_record);
                                let consumed = input.len() - output.len();
                                return consumed;
                            }
                        }
                    }
                    // no SMB3 here yet, will buffer full records
                }
            }
        }
        return 0;
    }

    /// Parsing function, handling TCP chunks fragmentation
    pub fn parse_tcp_data_tc<'b>(&mut self, flow: *const Flow, stream_slice: &StreamSlice) -> AppLayerResult
    {
        let mut cur_i = stream_slice.as_slice();
        let consumed = self.handle_skip(Direction::ToClient, cur_i.len() as u32);
        if consumed > 0 {
            if consumed > cur_i.len() as u32 {
                self.set_event(SMBEvent::InternalError);
                return AppLayerResult::err();
            }
            cur_i = &cur_i[consumed as usize..];
        }
        // take care of in progress file chunk transfers
        // and skip buffer beyond it
        let consumed = self.filetracker_update(Direction::ToClient, cur_i, 0);
        if consumed > 0 {
            if consumed > cur_i.len() as u32 {
                self.set_event(SMBEvent::InternalError);
                return AppLayerResult::err();
            }
            cur_i = &cur_i[consumed as usize..];
        }
        if cur_i.len() == 0 {
            return AppLayerResult::ok();
        }
        // gap
        if self.tc_gap {
            SCLogDebug!("TC trying to catch up after GAP (input {})", cur_i.len());
            while cur_i.len() > 0 { // min record size
                match search_smb_record(cur_i) {
                    Ok((_, pg)) => {
                        SCLogDebug!("smb record found");
                        let smb2_offset = cur_i.len() - pg.len();
                        if smb2_offset < 4 {
                            cur_i = &cur_i[smb2_offset+4..];
                            continue; // see if we have another record in our data
                        }
                        let nbss_offset = smb2_offset - 4;
                        cur_i = &cur_i[nbss_offset..];

                        self.tc_gap = false;
                        break;
                    },
                    _ => {
                        let mut consumed = stream_slice.len();
                        if consumed < 4 {
                            consumed = 0;
                        } else {
                            consumed = consumed - 3;
                        }
                        SCLogDebug!("smb record NOT found");
                        return AppLayerResult::incomplete(consumed as u32, 8);
                    },
                }
            }
        }
        while cur_i.len() > 0 { // min record size
            match parse_nbss_record(cur_i) {
                Ok((rem, ref nbss_hdr)) => {
                    SCLogDebug!("nbss record offset {} len {}", stream_slice.offset_from(cur_i), cur_i.len() - rem.len());
                    self.add_nbss_tc_frames(flow, stream_slice, cur_i, nbss_hdr.length as i64);
                    SCLogDebug!("nbss frames added");

                    if nbss_hdr.message_type == NBSS_MSGTYPE_SESSION_MESSAGE {
                        // we have the full records size worth of data,
                        // let's parse it
                        match parse_smb_version(nbss_hdr.data) {
                            Ok((_, ref smb)) => {
                                SCLogDebug!("SMB {:?}", smb);
                                if smb.version == 0xff_u8 { // SMB1
                                    SCLogDebug!("SMBv1 record");
                                    match parse_smb_record(nbss_hdr.data) {
                                        Ok((_, ref smb_record)) => {
                                            let pdu_frame = self.add_smb1_tc_pdu_frame(flow, stream_slice, nbss_hdr.data, nbss_hdr.length as i64);
                                            self.add_smb1_tc_hdr_data_frames(flow, stream_slice, nbss_hdr.data, nbss_hdr.length as i64);
                                            if smb_record.is_response() {
                                                smb1_response_record(self, smb_record);
                                            } else {
                                                SCLogDebug!("SMB1 request seen from server to client");
                                                if let Some(frame) = pdu_frame {
                                                    frame.add_event(flow, 1, SMBEvent::RequestToClient as u8);
                                                }
                                            }
                                        },
                                        _ => {
                                            self.set_event(SMBEvent::MalformedData);
                                            return AppLayerResult::err();
                                        },
                                    }
                                } else if smb.version == 0xfe_u8 { // SMB2
                                    let mut nbss_data = nbss_hdr.data;
                                    while nbss_data.len() > 0 {
                                        SCLogDebug!("SMBv2 record");
                                        match parse_smb2_response_record(nbss_data) {
                                            Ok((nbss_data_rem, ref smb_record)) => {
                                                let record_len = (nbss_data.len() - nbss_data_rem.len()) as i64;
                                                let pdu_frame = self.add_smb2_tc_pdu_frame(flow, stream_slice, nbss_data, record_len);
                                                self.add_smb2_tc_hdr_data_frames(flow, stream_slice, nbss_data, record_len, smb_record.header_len as i64);
                                                if smb_record.is_response() {
                                                    smb2_response_record(self, smb_record);
                                                } else {
                                                    SCLogDebug!("SMB2 request seen from server to client");
                                                    if let Some(frame) = pdu_frame {
                                                        frame.add_event(flow, 1, SMBEvent::RequestToClient as u8);
                                                    }
                                                }
                                                nbss_data = nbss_data_rem;
                                            },
                                            _ => {
                                                self.set_event(SMBEvent::MalformedData);
                                                return AppLayerResult::err();
                                            },
                                        }
                                    }
                                } else if smb.version == 0xfd_u8 { // SMB3 transform
                                    let mut nbss_data = nbss_hdr.data;
                                    while nbss_data.len() > 0 {
                                        SCLogDebug!("SMBv3 transform record");
                                        match parse_smb3_transform_record(nbss_data) {
                                            Ok((nbss_data_rem, ref _smb3_record)) => {
                                                let record_len = (nbss_data.len() - nbss_data_rem.len()) as i64;
                                                self.add_smb3_tc_pdu_frame(flow, stream_slice, nbss_data, record_len);
                                                self.add_smb3_tc_hdr_data_frames(flow, stream_slice, nbss_data, record_len);
                                                nbss_data = nbss_data_rem;
                                            },
                                            _ => {
                                                self.set_event(SMBEvent::MalformedData);
                                                return AppLayerResult::err();
                                            },
                                        }
                                    }
                                }
                            },
                            Err(Err::Incomplete(_)) => {
                                // not enough data to contain basic SMB hdr
                                // TODO event: empty NBSS_MSGTYPE_SESSION_MESSAGE
                            },
                            Err(_) => {
                                self.set_event(SMBEvent::MalformedData);
                                return AppLayerResult::err();
                            },
                        }
                    } else {
                        SCLogDebug!("NBSS message {:X}", nbss_hdr.message_type);
                    }
                    cur_i = rem;
                },
                Err(Err::Incomplete(needed)) => {
                    SCLogDebug!("INCOMPLETE have {} needed {:?}", cur_i.len(), needed);
                    if let Needed::Size(n) = needed {
                        let n = usize::from(n) + cur_i.len();
                        // 512 is the minimum for parse_tcp_data_tc_partial
                        if n >= 512 && cur_i.len() < 512 {
                            let total_consumed = stream_slice.offset_from(cur_i);
                            return AppLayerResult::incomplete(total_consumed, 512);
                        }
                        let consumed = self.parse_tcp_data_tc_partial(flow, stream_slice, cur_i);
                        if consumed == 0 {
                            // if we consumed none we will buffer the entire record
                            let total_consumed = stream_slice.offset_from(cur_i);
                            SCLogDebug!("setting consumed {} need {} needed {:?} total input {}",
                                    total_consumed, n, needed, stream_slice.len());
                            let need = n;
                            return AppLayerResult::incomplete(total_consumed as u32, need as u32);
                        }
                        // tracking a read record, which we don't need to
                        // queue up at the stream level, but can feed to us
                        // in small chunks
                        return AppLayerResult::ok();
                    } else {
                        self.set_event(SMBEvent::InternalError);
                        return AppLayerResult::err();
                    }
                },
                Err(_) => {
                    self.set_event(SMBEvent::MalformedData);
                    return AppLayerResult::err();
                },
            }
        };
        self.post_gap_housekeeping(Direction::ToClient);
        if self.check_post_gap_file_txs && !self.post_gap_files_checked {
            self.post_gap_housekeeping_for_files();
            self.post_gap_files_checked = true;
        }
        self._debug_tx_stats();
        AppLayerResult::ok()
    }

    /// handle a gap in the TOSERVER direction
    /// returns: 0 ok, 1 unrecoverable error
    pub fn parse_tcp_data_ts_gap(&mut self, gap_size: u32) -> AppLayerResult {
        let consumed = self.handle_skip(Direction::ToServer, gap_size);
        if consumed < gap_size {
            let new_gap_size = gap_size - consumed;
            let gap = vec![0; new_gap_size as usize];

            let consumed2 = self.filetracker_update(Direction::ToServer, &gap, new_gap_size);
            if consumed2 > new_gap_size {
                SCLogDebug!("consumed more than GAP size: {} > {}", consumed2, new_gap_size);
                self.set_event(SMBEvent::InternalError);
                return AppLayerResult::err();
            }
        }
        SCLogDebug!("GAP of size {} in toserver direction", gap_size);
        self.ts_ssn_gap = true;
        self.ts_gap = true;
        return AppLayerResult::ok();
    }

    /// handle a gap in the TOCLIENT direction
    /// returns: 0 ok, 1 unrecoverable error
    pub fn parse_tcp_data_tc_gap(&mut self, gap_size: u32) -> AppLayerResult {
        let consumed = self.handle_skip(Direction::ToClient, gap_size);
        if consumed < gap_size {
            let new_gap_size = gap_size - consumed;
            let gap = vec![0; new_gap_size as usize];

            let consumed2 = self.filetracker_update(Direction::ToClient, &gap, new_gap_size);
            if consumed2 > new_gap_size {
                SCLogDebug!("consumed more than GAP size: {} > {}", consumed2, new_gap_size);
                self.set_event(SMBEvent::InternalError);
                return AppLayerResult::err();
            }
        }
        SCLogDebug!("GAP of size {} in toclient direction", gap_size);
        self.tc_ssn_gap = true;
        self.tc_gap = true;
        return AppLayerResult::ok();
    }

    pub fn trunc_ts(&mut self) {
        SCLogDebug!("TRUNC TS");
        self.ts_trunc = true;

        for tx in &mut self.transactions {
            if !tx.request_done {
                SCLogDebug!("TRUNCING TX {} in TOSERVER direction", tx.id);
                tx.request_done = true;
            }
       }
    }
    pub fn trunc_tc(&mut self) {
        SCLogDebug!("TRUNC TC");
        self.tc_trunc = true;

        for tx in &mut self.transactions {
            if !tx.response_done {
                SCLogDebug!("TRUNCING TX {} in TOCLIENT direction", tx.id);
                tx.response_done = true;
            }
        }
    }
}

/// Returns *mut SMBState
#[no_mangle]
pub extern "C" fn rs_smb_state_new(_orig_state: *mut std::os::raw::c_void, _orig_proto: AppProto) -> *mut std::os::raw::c_void {
    let state = SMBState::new();
    let boxed = Box::new(state);
    SCLogDebug!("allocating state");
    return Box::into_raw(boxed) as *mut _;
}

/// Params:
/// - state: *mut SMBState as void pointer
#[no_mangle]
pub extern "C" fn rs_smb_state_free(state: *mut std::os::raw::c_void) {
    SCLogDebug!("freeing state");
    let mut smb_state = unsafe { Box::from_raw(state as *mut SMBState) };
    smb_state.free();
}

/// C binding parse a SMB request. Returns 1 on success, -1 on failure.
#[no_mangle]
pub unsafe extern "C" fn rs_smb_parse_request_tcp(flow: *const Flow,
                                       state: *mut ffi::c_void,
                                       _pstate: *mut std::os::raw::c_void,
                                       stream_slice: StreamSlice,
                                       _data: *const std::os::raw::c_void,
                                       )
                                       -> AppLayerResult
{
    let mut state = cast_pointer!(state, SMBState);
    let flow = cast_pointer!(flow, Flow);
    let file_flags = FileFlowToFlags(flow, Direction::ToServer as u8);
    rs_smb_setfileflags(Direction::ToServer as u8, state, file_flags|FILE_USE_DETECT);

    if stream_slice.is_gap() {
        return rs_smb_parse_request_tcp_gap(state, stream_slice.gap_size());
    }

    SCLogDebug!("parsing {} bytes of request data", stream_slice.len());

    /* START with MISTREAM set: record might be starting the middle. */
    if stream_slice.flags() & (STREAM_START|STREAM_MIDSTREAM) == (STREAM_START|STREAM_MIDSTREAM) {
        state.ts_gap = true;
    }

    state.update_ts(flow.get_last_time().as_secs());
    state.parse_tcp_data_ts(flow, &stream_slice)
}

#[no_mangle]
pub extern "C" fn rs_smb_parse_request_tcp_gap(
                                        state: &mut SMBState,
                                        input_len: u32)
                                        -> AppLayerResult
{
    state.parse_tcp_data_ts_gap(input_len as u32)
}


#[no_mangle]
pub unsafe extern "C" fn rs_smb_parse_response_tcp(flow: *const Flow,
                                        state: *mut ffi::c_void,
                                        _pstate: *mut std::os::raw::c_void,
                                        stream_slice: StreamSlice,
                                        _data: *const ffi::c_void,
                                        )
                                        -> AppLayerResult
{
    let mut state = cast_pointer!(state, SMBState);
    let flow = cast_pointer!(flow, Flow);
    let file_flags = FileFlowToFlags(flow, Direction::ToClient as u8);
    rs_smb_setfileflags(Direction::ToClient as u8, state, file_flags|FILE_USE_DETECT);

    if stream_slice.is_gap() {
        return rs_smb_parse_response_tcp_gap(state, stream_slice.gap_size());
    }

    /* START with MISTREAM set: record might be starting the middle. */
    if stream_slice.flags() & (STREAM_START|STREAM_MIDSTREAM) == (STREAM_START|STREAM_MIDSTREAM) {
        state.tc_gap = true;
    }

    state.update_ts(flow.get_last_time().as_secs());
    state.parse_tcp_data_tc(flow, &stream_slice)
}

#[no_mangle]
pub extern "C" fn rs_smb_parse_response_tcp_gap(
                                        state: &mut SMBState,
                                        input_len: u32)
                                        -> AppLayerResult
{
    state.parse_tcp_data_tc_gap(input_len as u32)
}

fn smb_probe_tcp_midstream(direction: Direction, slice: &[u8], rdir: *mut u8, begins: bool) -> i8
{
    let r = if begins {
        // if pattern was found in the beginning, just check first byte
        if slice[0] == NBSS_MSGTYPE_SESSION_MESSAGE {
            Ok((&slice[..4], &slice[4..]))
        } else {
            Err(Err::Error(make_error(slice, ErrorKind::Eof)))
        }
    } else {
        search_smb_record(slice)
    };
    match r {
        Ok((_, data)) => {
            SCLogDebug!("smb found");
            match parse_smb_version(data) {
                Ok((_, ref smb)) => {
                    SCLogDebug!("SMB {:?}", smb);
                    if smb.version == 0xff_u8 { // SMB1
                        SCLogDebug!("SMBv1 record");
                        match parse_smb_record(data) {
                            Ok((_, ref smb_record)) => {
                                if smb_record.flags & 0x80 != 0 {
                                    SCLogDebug!("RESPONSE {:02x}", smb_record.flags);
                                    if direction == Direction::ToServer {
                                        unsafe { *rdir = Direction::ToClient as u8; }
                                    }
                                } else {
                                    SCLogDebug!("REQUEST {:02x}", smb_record.flags);
                                    if direction == Direction::ToClient {
                                        unsafe { *rdir = Direction::ToServer as u8; }
                                    }
                                }
                                return 1;
                            },
                            _ => { },
                        }
                    } else if smb.version == 0xfe_u8 { // SMB2
                        SCLogDebug!("SMB2 record");
                        match parse_smb2_record_direction(data) {
                            Ok((_, ref smb_record)) => {
                                if direction == Direction::ToServer {
                                    SCLogDebug!("direction Direction::ToServer smb_record {:?}", smb_record);
                                    if !smb_record.request {
                                        unsafe { *rdir = Direction::ToClient as u8; }
                                    }
                                } else {
                                    SCLogDebug!("direction Direction::ToClient smb_record {:?}", smb_record);
                                    if smb_record.request {
                                        unsafe { *rdir = Direction::ToServer as u8; }
                                    }
                                }
                            },
                            _ => {},
                        }
                    }
                    else if smb.version == 0xfd_u8 { // SMB3 transform
                        SCLogDebug!("SMB3 record");
                    }
                    return 1;
                },
                    _ => {
                        SCLogDebug!("smb not found in {:?}", slice);
                    },
            }
        },
        _ => {
            SCLogDebug!("no dice");
        },
    }
    return 0;
}

fn smb_probe_tcp(flags: u8, slice: &[u8], rdir: *mut u8, begins: bool) -> AppProto
{
    if flags & STREAM_MIDSTREAM == STREAM_MIDSTREAM {
        if smb_probe_tcp_midstream(flags.into(), slice, rdir, begins) == 1 {
            unsafe { return ALPROTO_SMB; }
        }
    }
    match parse_nbss_record_partial(slice) {
        Ok((_, ref hdr)) => {
            if hdr.is_smb() {
                SCLogDebug!("smb found");
                unsafe { return ALPROTO_SMB; }
            } else if hdr.needs_more(){
                return 0;
            } else if hdr.is_valid() &&
                hdr.message_type != NBSS_MSGTYPE_SESSION_MESSAGE {
                //we accept a first small netbios message before real SMB
                let hl = hdr.length as usize;
                if hdr.data.len() >= hl + 8 {
                    // 8 is 4 bytes NBSS + 4 bytes SMB0xFX magic
                    match parse_nbss_record_partial(&hdr.data[hl..]) {
                        Ok((_, ref hdr2)) => {
                            if hdr2.is_smb() {
                                SCLogDebug!("smb found");
                                unsafe { return ALPROTO_SMB; }
                            }
                        }
                        _ => {}
                    }
                } else if hdr.length < 256 {
                    // we want more data, 256 is some random value
                    return 0;
                }
                // default is failure
            }
        },
        _ => { },
    }
    SCLogDebug!("no smb");
    unsafe { return ALPROTO_FAILED; }
}

// probing confirmation parser
// return 1 if found, 0 is not found
#[no_mangle]
pub unsafe extern "C" fn rs_smb_probe_begins_tcp(_f: *const Flow,
                                   flags: u8, input: *const u8, len: u32, rdir: *mut u8)
    -> AppProto
{
    if len < MIN_REC_SIZE as u32 {
        return ALPROTO_UNKNOWN;
    }
    let slice = build_slice!(input, len as usize);
    return smb_probe_tcp(flags, slice, rdir, true);
}

// probing parser
// return 1 if found, 0 is not found
#[no_mangle]
pub unsafe extern "C" fn rs_smb_probe_tcp(_f: *const Flow,
                                   flags: u8, input: *const u8, len: u32, rdir: *mut u8)
    -> AppProto
{
    if len < MIN_REC_SIZE as u32 {
        return ALPROTO_UNKNOWN;
    }
    let slice = build_slice!(input, len as usize);
    return smb_probe_tcp(flags, slice, rdir, false);
}

#[no_mangle]
pub unsafe extern "C" fn rs_smb_state_get_tx_count(state: *mut ffi::c_void)
                                            -> u64
{
    let state = cast_pointer!(state, SMBState);
    SCLogDebug!("rs_smb_state_get_tx_count: returning {}", state.tx_id);
    return state.tx_id;
}

#[no_mangle]
pub unsafe extern "C" fn rs_smb_state_get_tx(state: *mut ffi::c_void,
                                      tx_id: u64)
                                      -> *mut ffi::c_void
{
    let state = cast_pointer!(state, SMBState);
    match state.get_tx_by_id(tx_id) {
        Some(tx) => {
            return tx as *const _ as *mut _;
        }
        None => {
            return std::ptr::null_mut();
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn rs_smb_state_tx_free(state: *mut ffi::c_void,
                                       tx_id: u64)
{
    let state = cast_pointer!(state, SMBState);
    SCLogDebug!("freeing tx {}", tx_id as u64);
    state.free_tx(tx_id);
}

#[no_mangle]
pub unsafe extern "C" fn rs_smb_tx_get_alstate_progress(tx: *mut ffi::c_void,
                                                  direction: u8)
                                                  -> i32
{
    let tx = cast_pointer!(tx, SMBTransaction);

    if direction == Direction::ToServer as u8 && tx.request_done {
        SCLogDebug!("tx {} TOSERVER progress 1 => {:?}", tx.id, tx);
        return 1;
    } else if direction == Direction::ToClient as u8 && tx.response_done {
        SCLogDebug!("tx {} TOCLIENT progress 1 => {:?}", tx.id, tx);
        return 1;
    } else {
        SCLogDebug!("tx {} direction {:?} progress 0", tx.id, direction);
        return 0;
    }
}


#[no_mangle]
pub unsafe extern "C" fn rs_smb_get_tx_data(
    tx: *mut std::os::raw::c_void)
    -> *mut AppLayerTxData
{
    let tx = cast_pointer!(tx, SMBTransaction);
    return &mut tx.tx_data;
}


#[no_mangle]
pub unsafe extern "C" fn rs_smb_state_truncate(
        state: *mut std::ffi::c_void,
        direction: u8)
{
    let state = cast_pointer!(state, SMBState);
    match direction.into() {
        Direction::ToServer => {
            state.trunc_ts();
        }
        Direction::ToClient => {
            state.trunc_tc();
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn rs_smb_state_get_event_info_by_id(
    event_id: std::os::raw::c_int,
    event_name: *mut *const std::os::raw::c_char,
    event_type: *mut AppLayerEventType,
) -> i8 {
    SMBEvent::get_event_info_by_id(event_id, event_name, event_type)
}

#[no_mangle]
pub unsafe extern "C" fn rs_smb_state_get_event_info(
    event_name: *const std::os::raw::c_char,
    event_id: *mut std::os::raw::c_int,
    event_type: *mut AppLayerEventType,
) -> std::os::raw::c_int {
    SMBEvent::get_event_info(event_name, event_id, event_type)
}

pub unsafe extern "C" fn smb3_probe_tcp(f: *const Flow, dir: u8, input: *const u8, len: u32, rdir: *mut u8) -> u16 {
    let retval = rs_smb_probe_tcp(f, dir, input, len, rdir);
    let f = cast_pointer!(f, Flow);
    if retval != ALPROTO_SMB {
        return retval;
    }
    let (sp, dp) = f.get_ports();
    let flags = f.get_flags();
    let fsp = if (flags & FLOW_DIR_REVERSED) != 0 { dp } else { sp };
    let fdp = if (flags & FLOW_DIR_REVERSED) != 0 { sp } else { dp };
    if fsp == 445 && fdp != 445 {
        match dir.into() {
            Direction::ToServer => {
                *rdir = Direction::ToClient as u8;
            }
            Direction::ToClient => {
                *rdir = Direction::ToServer as u8;
            }
        }
    }
    return ALPROTO_SMB;
}

fn register_pattern_probe() -> i8 {
    let mut r = 0;
    unsafe {
        // SMB1
        r |= AppLayerProtoDetectPMRegisterPatternCSwPP(IPPROTO_TCP as u8, ALPROTO_SMB,
                                                     b"|ff|SMB\0".as_ptr() as *const std::os::raw::c_char, 8, 4,
                                                     Direction::ToServer as u8, rs_smb_probe_begins_tcp, MIN_REC_SIZE, MIN_REC_SIZE);
        r |= AppLayerProtoDetectPMRegisterPatternCSwPP(IPPROTO_TCP as u8, ALPROTO_SMB,
                                                     b"|ff|SMB\0".as_ptr() as *const std::os::raw::c_char, 8, 4,
                                                     Direction::ToClient as u8, rs_smb_probe_begins_tcp, MIN_REC_SIZE, MIN_REC_SIZE);
        // SMB2/3
        r |= AppLayerProtoDetectPMRegisterPatternCSwPP(IPPROTO_TCP as u8, ALPROTO_SMB,
                                                     b"|fe|SMB\0".as_ptr() as *const std::os::raw::c_char, 8, 4,
                                                     Direction::ToServer as u8, rs_smb_probe_begins_tcp, MIN_REC_SIZE, MIN_REC_SIZE);
        r |= AppLayerProtoDetectPMRegisterPatternCSwPP(IPPROTO_TCP as u8, ALPROTO_SMB,
                                                     b"|fe|SMB\0".as_ptr() as *const std::os::raw::c_char, 8, 4,
                                                     Direction::ToClient as u8, rs_smb_probe_begins_tcp, MIN_REC_SIZE, MIN_REC_SIZE);
        // SMB3 encrypted records
        r |= AppLayerProtoDetectPMRegisterPatternCSwPP(IPPROTO_TCP as u8, ALPROTO_SMB,
                                                     b"|fd|SMB\0".as_ptr() as *const std::os::raw::c_char, 8, 4,
                                                     Direction::ToServer as u8, smb3_probe_tcp, MIN_REC_SIZE, MIN_REC_SIZE);
        r |= AppLayerProtoDetectPMRegisterPatternCSwPP(IPPROTO_TCP as u8, ALPROTO_SMB,
                                                     b"|fd|SMB\0".as_ptr() as *const std::os::raw::c_char, 8, 4,
                                                     Direction::ToClient as u8, smb3_probe_tcp, MIN_REC_SIZE, MIN_REC_SIZE);
    }

    if r == 0 {
        return 0;
    } else {
        return -1;
    }
}

// Parser name as a C style string.
const PARSER_NAME: &'static [u8] = b"smb\0";

#[no_mangle]
pub unsafe extern "C" fn rs_smb_register_parser() {
    let default_port = CString::new("445").unwrap();
    let mut stream_depth = SMB_CONFIG_DEFAULT_STREAM_DEPTH;
    let parser = RustParser {
        name: PARSER_NAME.as_ptr() as *const std::os::raw::c_char,
        default_port: std::ptr::null(),
        ipproto: IPPROTO_TCP,
        probe_ts: None,
        probe_tc: None,
        min_depth: 0,
        max_depth: 16,
        state_new: rs_smb_state_new,
        state_free: rs_smb_state_free,
        tx_free: rs_smb_state_tx_free,
        parse_ts: rs_smb_parse_request_tcp,
        parse_tc: rs_smb_parse_response_tcp,
        get_tx_count: rs_smb_state_get_tx_count,
        get_tx: rs_smb_state_get_tx,
        tx_comp_st_ts: 1,
        tx_comp_st_tc: 1,
        tx_get_progress: rs_smb_tx_get_alstate_progress,
        get_eventinfo: Some(rs_smb_state_get_event_info),
        get_eventinfo_byid : Some(rs_smb_state_get_event_info_by_id),
        localstorage_new: None,
        localstorage_free: None,
        get_files: Some(rs_smb_getfiles),
        get_tx_iterator: Some(applayer::state_get_tx_iterator::<SMBState, SMBTransaction>),
        get_tx_data: rs_smb_get_tx_data,
        apply_tx_config: None,
        flags: APP_LAYER_PARSER_OPT_ACCEPT_GAPS,
        truncate: Some(rs_smb_state_truncate),
        get_frame_id_by_name: Some(SMBFrameType::ffi_id_from_name),
        get_frame_name_by_id: Some(SMBFrameType::ffi_name_from_id),
    };

    let ip_proto_str = CString::new("tcp").unwrap();

    if AppLayerProtoDetectConfProtoDetectionEnabled(
        ip_proto_str.as_ptr(),
        parser.name,
    ) != 0
    {
        let alproto = AppLayerRegisterProtocolDetection(&parser, 1);
        ALPROTO_SMB = alproto;
        if register_pattern_probe() < 0 {
            return;
        }

        let have_cfg = AppLayerProtoDetectPPParseConfPorts(ip_proto_str.as_ptr(),
                    IPPROTO_TCP as u8, parser.name, ALPROTO_SMB, 0,
                    MIN_REC_SIZE, rs_smb_probe_tcp, rs_smb_probe_tcp);

        if have_cfg == 0 {
            AppLayerProtoDetectPPRegister(IPPROTO_TCP as u8, default_port.as_ptr(), ALPROTO_SMB,
                                          0, MIN_REC_SIZE, Direction::ToServer as u8, rs_smb_probe_tcp, rs_smb_probe_tcp);
        }

        if AppLayerParserConfParserEnabled(
            ip_proto_str.as_ptr(),
            parser.name,
        ) != 0
        {
            let _ = AppLayerRegisterParser(&parser, alproto);
        }
        SCLogDebug!("Rust SMB parser registered.");
        let retval = conf_get("app-layer.protocols.smb.stream-depth");
        if let Some(val) = retval {
            match get_memval(val) {
                Ok(retval) => { stream_depth = retval as u32; }
                Err(_) => { SCLogError!("Invalid depth value"); }
            }
        }
        AppLayerParserSetStreamDepth(IPPROTO_TCP as u8, ALPROTO_SMB, stream_depth);
        let retval = conf_get("app-layer.protocols.smb.max-read-size");
        if let Some(val) = retval {
            match get_memval(val) {
                Ok(retval) => { SMB_CFG_MAX_READ_SIZE = retval as u32; }
                Err(_) => { SCLogError!("Invalid max-read-size value"); }
            }
        }
        let retval = conf_get("app-layer.protocols.smb.max-write-size");
        if let Some(val) = retval {
            match get_memval(val) {
                Ok(retval) => { SMB_CFG_MAX_WRITE_SIZE = retval as u32; }
                Err(_) => { SCLogError!("Invalid max-write-size value"); }
            }
        }
        let retval = conf_get("app-layer.protocols.smb.max-write-queue-size");
        if let Some(val) = retval {
            match get_memval(val) {
                Ok(retval) => { SMB_CFG_MAX_WRITE_QUEUE_SIZE = retval as u32; }
                Err(_) => { SCLogError!("Invalid max-write-queue-size value"); }
            }
        }
        let retval = conf_get("app-layer.protocols.smb.max-write-queue-cnt");
        if let Some(val) = retval {
            match get_memval(val) {
                Ok(retval) => { SMB_CFG_MAX_WRITE_QUEUE_CNT = retval as u32; }
                Err(_) => { SCLogError!("Invalid max-write-queue-cnt value"); }
            }
        }
        let retval = conf_get("app-layer.protocols.smb.max-read-queue-size");
        if let Some(val) = retval {
            match get_memval(val) {
                Ok(retval) => { SMB_CFG_MAX_READ_QUEUE_SIZE = retval as u32; }
                Err(_) => { SCLogError!("Invalid max-read-queue-size value"); }
            }
        }
        let retval = conf_get("app-layer.protocols.smb.max-read-queue-cnt");
        if let Some(val) = retval {
            match get_memval(val) {
                Ok(retval) => { SMB_CFG_MAX_READ_QUEUE_CNT = retval as u32; }
                Err(_) => { SCLogError!("Invalid max-read-queue-cnt value"); }
            }
        }
    } else {
        SCLogDebug!("Protocol detector and parser disabled for SMB.");
    }
}
