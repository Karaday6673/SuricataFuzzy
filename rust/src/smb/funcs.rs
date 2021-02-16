/* Copyright (C) 2017 Open Information Security Foundation
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

// Based on the list in Wiresharks packet-smb2.c
// Names match names from Microsoft.

pub fn fsctl_func_to_string(f: u32) -> String {
    match f {
        0x00060194 => "FSCTL_DFS_GET_REFERRALS",
        0x000601B0 => "FSCTL_DFS_GET_REFERRALS_EX",
        0x00090000 => "FSCTL_REQUEST_OPLOCK_LEVEL_1",
        0x00090004 => "FSCTL_REQUEST_OPLOCK_LEVEL_2",
        0x00090008 => "FSCTL_REQUEST_BATCH_OPLOCK",
        0x0009000C => "FSCTL_OPLOCK_BREAK_ACKNOWLEDGE",
        0x00090010 => "FSCTL_OPBATCH_ACK_CLOSE_PENDING",
        0x00090014 => "FSCTL_OPLOCK_BREAK_NOTIFY",
        0x00090018 => "FSCTL_LOCK_VOLUME",
        0x0009001C => "FSCTL_UNLOCK_VOLUME",
        0x00090020 => "FSCTL_DISMOUNT_VOLUME",
        0x00090028 => "FSCTL_IS_VOLUME_MOUNTED",
        0x0009002C => "FSCTL_IS_PATHNAME_VALID",
        0x00090030 => "FSCTL_MARK_VOLUME_DIRTY",
        0x0009003B => "FSCTL_QUERY_RETRIEVAL_POINTERS",
        0x0009003C => "FSCTL_GET_COMPRESSION",
        0x0009004F => "FSCTL_MARK_AS_SYSTEM_HIVE",
        0x00090050 => "FSCTL_OPLOCK_BREAK_ACK_NO_2",
        0x00090054 => "FSCTL_INVALIDATE_VOLUMES",
        0x00090058 => "FSCTL_QUERY_FAT_BPB",
        0x0009005C => "FSCTL_REQUEST_FILTER_OPLOCK",
        0x00090060 => "FSCTL_FILESYSTEM_GET_STATISTICS",
        0x00090064 => "FSCTL_GET_NTFS_VOLUME_DATA",
        0x00090068 => "FSCTL_GET_NTFS_FILE_RECORD",
        0x0009006F => "FSCTL_GET_VOLUME_BITMAP",
        0x00090073 => "FSCTL_GET_RETRIEVAL_POINTERS",
        0x00090074 => "FSCTL_MOVE_FILE",
        0x00090078 => "FSCTL_IS_VOLUME_DIRTY",
        0x0009007C => "FSCTL_GET_HFS_INFORMATION",
        0x00090083 => "FSCTL_ALLOW_EXTENDED_DASD_IO",
        0x00090087 => "FSCTL_READ_PROPERTY_DATA",
        0x0009008B => "FSCTL_WRITE_PROPERTY_DATA",
        0x0009008F => "FSCTL_FIND_FILES_BY_SID",
        0x00090097 => "FSCTL_DUMP_PROPERTY_DATA",
        0x0009009C => "FSCTL_GET_OBJECT_ID",
        0x000900A4 => "FSCTL_SET_REPARSE_POINT",
        0x000900A8 => "FSCTL_GET_REPARSE_POINT",
        0x000900C0 => "FSCTL_CREATE_OR_GET_OBJECT_ID",
        0x000900C4 => "FSCTL_SET_SPARSE",
        0x000900D4 => "FSCTL_SET_ENCRYPTION",
        0x000900DB => "FSCTL_ENCRYPTION_FSCTL_IO",
        0x000900DF => "FSCTL_WRITE_RAW_ENCRYPTED",
        0x000900E3 => "FSCTL_READ_RAW_ENCRYPTED",
        0x000900F0 => "FSCTL_EXTEND_VOLUME",
        0x00090244 => "FSCTL_CSV_TUNNEL_REQUEST",
        0x0009027C => "FSCTL_GET_INTEGRITY_INFORMATION",
        0x00090284 => "FSCTL_QUERY_FILE_REGIONS",
        0x000902c8 => "FSCTL_CSV_SYNC_TUNNEL_REQUEST",
        0x00090300 => "FSCTL_QUERY_SHARED_VIRTUAL_DISK_SUPPORT",
        0x00090304 => "FSCTL_SVHDX_SYNC_TUNNEL_REQUEST",
        0x00090308 => "FSCTL_SVHDX_SET_INITIATOR_INFORMATION",
        0x0009030C => "FSCTL_SET_EXTERNAL_BACKING",
        0x00090310 => "FSCTL_GET_EXTERNAL_BACKING",
        0x00090314 => "FSCTL_DELETE_EXTERNAL_BACKING",
        0x00090318 => "FSCTL_ENUM_EXTERNAL_BACKING",
        0x0009031F => "FSCTL_ENUM_OVERLAY",
        0x00090350 => "FSCTL_STORAGE_QOS_CONTROL",
        0x00090364 => "FSCTL_SVHDX_ASYNC_TUNNEL_REQUEST",
        0x000940B3 => "FSCTL_ENUM_USN_DATA",
        0x000940B7 => "FSCTL_SECURITY_ID_CHECK",
        0x000940BB => "FSCTL_READ_USN_JOURNAL",
        0x000940CF => "FSCTL_QUERY_ALLOCATED_RANGES",
        0x000940E7 => "FSCTL_CREATE_USN_JOURNAL",
        0x000940EB => "FSCTL_READ_FILE_USN_DATA",
        0x000940EF => "FSCTL_WRITE_USN_CLOSE_RECORD",
        0x00094264 => "FSCTL_OFFLOAD_READ",
        0x00098098 => "FSCTL_SET_OBJECT_ID",
        0x000980A0 => "FSCTL_DELETE_OBJECT_ID",
        0x000980A4 => "FSCTL_SET_REPARSE_POINT",
        0x000980AC => "FSCTL_DELETE_REPARSE_POINT",
        0x000980BC => "FSCTL_SET_OBJECT_ID_EXTENDED",
        0x000980C8 => "FSCTL_SET_ZERO_DATA",
        0x000980D0 => "FSCTL_ENABLE_UPGRADE",
        0x00098208 => "FSCTL_FILE_LEVEL_TRIM",
        0x00098268 => "FSCTL_OFFLOAD_WRITE",
        0x0009C040 => "FSCTL_SET_COMPRESSION",
        0x0009C280 => "FSCTL_SET_INTEGRITY_INFORMATION",
        0x00110018 => "FSCTL_PIPE_WAIT",
        0x0011400C => "FSCTL_PIPE_PEEK",
        0x0011C017 => "FSCTL_PIPE_TRANSCEIVE",
        0x00140078 => "FSCTL_SRV_REQUEST_RESUME_KEY",
        0x001401D4 => "FSCTL_LMR_REQUEST_RESILIENCY",
        0x001401FC => "FSCTL_QUERY_NETWORK_INTERFACE_INFO",
        0x00140200 => "FSCTL_VALIDATE_NEGOTIATE_INFO_224",
        0x00140204 => "FSCTL_VALIDATE_NEGOTIATE_INFO",
        0x00144064 => "FSCTL_SRV_ENUMERATE_SNAPSHOTS",
        0x001440F2 => "FSCTL_SRV_COPYCHUNK",
        0x001441bb => "FSCTL_SRV_READ_HASH",
        0x001480F2 => "FSCTL_SRV_COPYCHUNK_WRITE",
        _ => { return (f).to_string(); },
    }.to_string()
}
