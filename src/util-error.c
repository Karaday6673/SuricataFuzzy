/* Copyright (C) 2007-2010 Open Information Security Foundation
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

/**
 * \file
 *
 * \author Anoop Saldanha <anoopsaldanha@gmail.com>
 *
 * Error utility functions
 *
 * \todo Needs refining of the error codes.  Renaming with a prefix of SC_ERR,
 *       removal of duplicates and entries have to be made in util-error.c
 */

#include "util-error.h"

#define CASE_CODE(E)  case E: return #E

/**
 * \brief Maps the error code, to its string equivalent
 *
 * \param The error code
 *
 * \retval The string equivalent for the error code
 */
const char * SCErrorToString(SCError err)
{
    switch (err) {
        CASE_CODE (SC_OK);
        CASE_CODE (SC_ERR_MEM_ALLOC);
        CASE_CODE (SC_ERR_ACTION_ORDER);
        CASE_CODE (SC_ERR_PCRE_MATCH);
        CASE_CODE (SC_ERR_PCRE_GET_SUBSTRING);
        CASE_CODE (SC_ERR_PCRE_COMPILE);
        CASE_CODE (SC_ERR_PCRE_STUDY);
        CASE_CODE (SC_ERR_PCRE_PARSE);
        CASE_CODE (SC_ERR_LOG_MODULE_NOT_INIT);
        CASE_CODE (SC_ERR_LOG_FG_FILTER_MATCH);
        CASE_CODE (SC_ERR_PCAP_DISPATCH);
        CASE_CODE (SC_ERR_PCAP_CREATE);
        CASE_CODE (SC_ERR_PCAP_SET_SNAPLEN);
        CASE_CODE (SC_ERR_PCAP_SET_PROMISC);
        CASE_CODE (SC_ERR_PCAP_SET_TIMEOUT);
        CASE_CODE (SC_ERR_PCAP_OPEN_LIVE);
        CASE_CODE (SC_ERR_PCAP_OPEN_OFFLINE);
        CASE_CODE (SC_ERR_PCAP_ACTIVATE_HANDLE);
        CASE_CODE (SC_ERR_PCAP_SET_BUFF_SIZE);
        CASE_CODE (SC_ERR_NO_PCAP_SET_BUFFER_SIZE);
        CASE_CODE (SC_ERR_NO_PF_RING);
        CASE_CODE (SC_ERR_PF_RING_RECV);
        CASE_CODE (SC_ERR_PF_RING_GET_CLUSTERID_FAILED);
        CASE_CODE (SC_ERR_PF_RING_GET_INTERFACE_FAILED);
        CASE_CODE (SC_ERR_PF_RING_OPEN);
        CASE_CODE (SC_ERR_GET_CLUSTER_TYPE_FAILED);
        CASE_CODE (SC_ERR_INVALID_CLUSTER_TYPE);
        CASE_CODE (SC_ERR_PF_RING_SET_CLUSTER_FAILED);
        CASE_CODE (SC_ERR_DATALINK_UNIMPLEMENTED);
        CASE_CODE (SC_ERR_INVALID_SIGNATURE);
        CASE_CODE (SC_ERR_OPENING_FILE);
        CASE_CODE (SC_ERR_OPENING_RULE_FILE);
        CASE_CODE (SC_ERR_NO_RULES);
        CASE_CODE (SC_ERR_NO_RULES_LOADED);
        CASE_CODE (SC_ERR_COUNTER_EXCEEDED);
        CASE_CODE (SC_ERR_INVALID_CHECKSUM);
        CASE_CODE (SC_ERR_SPRINTF);
        CASE_CODE (SC_ERR_FATAL);
        CASE_CODE (SC_ERR_INVALID_ARGUMENT);
        CASE_CODE (SC_ERR_SPINLOCK);
        CASE_CODE (SC_ERR_INVALID_ENUM_MAP);
        CASE_CODE (SC_ERR_INVALID_IP_NETBLOCK);
        CASE_CODE (SC_ERR_INVALID_IPV4_ADDR);
        CASE_CODE (SC_ERR_INVALID_IPV6_ADDR);
        CASE_CODE (SC_ERR_INVALID_RUNMODE);
        CASE_CODE (SC_ERR_COMPLETE_PORT_SPACE_NEGATED);
        CASE_CODE (SC_ERR_NO_PORTS_LEFT_AFTER_MERGE);
        CASE_CODE (SC_ERR_NEGATED_VALUE_IN_PORT_RANGE);
        CASE_CODE (SC_ERR_PORT_PARSE_INSERT_STRING);
        CASE_CODE (SC_ERR_UNREACHABLE_CODE_REACHED);
        CASE_CODE (SC_ERR_INVALID_NUMERIC_VALUE);
        CASE_CODE (SC_ERR_NUMERIC_VALUE_ERANGE);
        CASE_CODE (SC_ERR_INVALID_NUM_BYTES);
        CASE_CODE (SC_ERR_ARG_LEN_LONG);
        CASE_CODE (SC_ERR_ALPARSER);
        CASE_CODE (SC_ERR_POOL_EMPTY);
        CASE_CODE (SC_ERR_REASSEMBLY);
        CASE_CODE (SC_ERR_POOL_INIT);
        CASE_CODE (SC_ERR_UNIMPLEMENTED);
        CASE_CODE (SC_ERR_ADDRESS_ENGINE_GENERIC);
        CASE_CODE (SC_ERR_PORT_ENGINE_GENERIC);
        CASE_CODE (SC_ERR_FAST_LOG_GENERIC);
        CASE_CODE (SC_ERR_IPONLY_RADIX);
        CASE_CODE (SC_ERR_DEBUG_LOG_GENERIC);
        CASE_CODE (SC_ERR_UNIFIED_LOG_GENERIC);
        CASE_CODE (SC_ERR_HTTP_LOG_GENERIC);
        CASE_CODE (SC_ERR_UNIFIED_ALERT_GENERIC);
        CASE_CODE (SC_ERR_UNIFIED2_ALERT_GENERIC);
        CASE_CODE (SC_ERR_FWRITE);
        CASE_CODE (SC_ERR_FOPEN);
        CASE_CODE (SC_ERR_THREAD_NICE_PRIO);
        CASE_CODE (SC_ERR_THREAD_SPAWN);
        CASE_CODE (SC_ERR_THREAD_CREATE);
        CASE_CODE (SC_ERR_THREAD_INIT);
        CASE_CODE (SC_ERR_THRESHOLD_HASH_ADD);
        CASE_CODE (SC_ERR_UNDEFINED_VAR);
        CASE_CODE (SC_ERR_RULE_KEYWORD_UNKNOWN);
        CASE_CODE (SC_ERR_FLAGS_MODIFIER);
        CASE_CODE (SC_ERR_DISTANCE_MISSING_CONTENT);
        CASE_CODE (SC_ERR_BYTETEST_MISSING_CONTENT);
        CASE_CODE (SC_ERR_BYTEJUMP_MISSING_CONTENT);
        CASE_CODE (SC_ERR_WITHIN_MISSING_CONTENT);
        CASE_CODE (SC_ERR_WITHIN_INVALID);
        CASE_CODE (SC_ERR_DEPTH_MISSING_CONTENT);
        CASE_CODE (SC_ERR_OFFSET_MISSING_CONTENT);
        CASE_CODE (SC_ERR_NOCASE_MISSING_PATTERN);
        CASE_CODE (SC_ERR_RAWBYTES_MISSING_CONTENT);
        CASE_CODE (SC_ERR_NO_URICONTENT_NEGATION);
        CASE_CODE (SC_ERR_HASH_TABLE_INIT);
        CASE_CODE (SC_ERR_STAT);
        CASE_CODE (SC_ERR_LOGDIR_CONFIG);
        CASE_CODE (SC_ERR_LOGDIR_CMDLINE);
        CASE_CODE (SC_ERR_RADIX_TREE_GENERIC);
        CASE_CODE (SC_ERR_MISSING_QUOTE);
        CASE_CODE (SC_ERR_UNKNOWN_PROTOCOL);
        CASE_CODE (SC_ERR_UNKNOWN_RUN_MODE);
        CASE_CODE (SC_ERR_IPFW_NOSUPPORT);
        CASE_CODE (SC_ERR_IPFW_BIND);
        CASE_CODE (SC_ERR_IPFW_SOCK);
        CASE_CODE (SC_ERR_IPFW_SETSOCKOPT);
        CASE_CODE (SC_ERR_IPFW_NOPORT);
        CASE_CODE (SC_WARN_IPFW_RECV);
        CASE_CODE (SC_WARN_IPFW_XMIT);
        CASE_CODE (SC_WARN_IPFW_SETSOCKOPT);
        CASE_CODE (SC_WARN_IPFW_UNBIND);
        CASE_CODE (SC_ERR_MULTIPLE_RUN_MODE);
        CASE_CODE (SC_ERR_BPF);
        CASE_CODE (SC_ERR_MISSING_CONFIG_PARAM);
        CASE_CODE (SC_ERR_UNKNOWN_VALUE);
        CASE_CODE (SC_ERR_INVALID_VALUE);
        CASE_CODE (SC_ERR_UNKNOWN_REGEX_MOD);
        CASE_CODE (SC_ERR_INVALID_OPERATOR);
        CASE_CODE (SC_ERR_PCAP_RECV_INIT);
        CASE_CODE (SC_ERR_NFQ_NOSUPPORT);
        CASE_CODE (SC_ERR_NFQ_UNBIND);
        CASE_CODE (SC_ERR_NFQ_BIND);
        CASE_CODE (SC_ERR_NFQ_HANDLE_PKT);
        CASE_CODE (SC_ERR_CUDA_ERROR);
        CASE_CODE (SC_ERR_CUDA_HANDLER_ERROR);
        CASE_CODE (SC_ERR_TM_THREADS_ERROR);
        CASE_CODE (SC_ERR_TM_MODULES_ERROR);
        CASE_CODE (SC_ERR_AC_CUDA_ERROR);
        CASE_CODE (SC_ERR_INVALID_YAML_CONF_ENTRY);
        CASE_CODE (SC_ERR_TMQ_ALREADY_REGISTERED);
        CASE_CODE (SC_ERR_CONFLICTING_RULE_KEYWORDS);
        CASE_CODE (SC_ERR_INITIALIZATION);
        CASE_CODE (SC_ERR_INVALID_ACTION);
        CASE_CODE (SC_ERR_LIBNET_REQUIRED_FOR_ACTION);
        CASE_CODE (SC_ERR_LIBNET_INIT);
        CASE_CODE (SC_ERR_LIBNET_INVALID_DIR);
        CASE_CODE (SC_ERR_LIBNET_BUILD_FAILED);
        CASE_CODE (SC_ERR_LIBNET_WRITE_FAILED);
        CASE_CODE (SC_ERR_LIBNET_NOT_ENABLED);
        CASE_CODE (SC_ERR_UNIFIED_LOG_FILE_HEADER);
        CASE_CODE (SC_ERR_REFERENCE_UNKNOWN);
        CASE_CODE (SC_ERR_PIDFILE_SNPRINTF);
        CASE_CODE (SC_ERR_PIDFILE_OPEN);
        CASE_CODE (SC_ERR_PIDFILE_WRITE);
        CASE_CODE (SC_ERR_PIDFILE_DAEMON);
        CASE_CODE (SC_ERR_UID_FAILED);
        CASE_CODE (SC_ERR_GID_FAILED);
        CASE_CODE (SC_ERR_CHANGING_CAPS_FAILED);
        CASE_CODE (SC_ERR_LIBCAP_NG_REQUIRED);
        CASE_CODE (SC_ERR_LIBNET11_INCOMPATIBLE_WITH_LIBCAP_NG);
        CASE_CODE (SC_WARN_FLOW_EMERGENCY);
        CASE_CODE (SC_ERR_SVC);
        CASE_CODE (SC_ERR_ERF_DAG_OPEN_FAILED);
        CASE_CODE (SC_ERR_ERF_DAG_STREAM_OPEN_FAILED);
        CASE_CODE (SC_ERR_ERF_DAG_STREAM_START_FAILED);
        CASE_CODE (SC_ERR_ERF_DAG_STREAM_SET_FAILED);
        CASE_CODE (SC_ERR_ERF_DAG_STREAM_READ_FAILED);
        CASE_CODE (SC_WARN_ERF_DAG_REC_LEN_CHANGED);
        CASE_CODE (SC_ERR_NAPATECH_OPEN_FAILED);
        CASE_CODE (SC_ERR_NAPATECH_STREAM_NEXT_FAILED);
        CASE_CODE (SC_ERR_NAPATECH_NOSUPPORT);
        CASE_CODE (SC_ERR_NAPATECH_REQUIRED);
        CASE_CODE (SC_ERR_NAPATECH_TIMESTAMP_TYPE_NOT_SUPPORTED);
        CASE_CODE (SC_ERR_NAPATECH_INIT_FAILED);
        CASE_CODE (SC_ERR_NAPATECH_CONFIG_STREAM);
        CASE_CODE (SC_ERR_NAPATECH_STREAMS_REGISTER_FAILED);
        CASE_CODE (SC_ERR_NAPATECH_STAT_DROPS_FAILED);
        CASE_CODE (SC_ERR_NAPATECH_PARSE_CONFIG);
        CASE_CODE (SC_WARN_COMPATIBILITY);
        CASE_CODE (SC_ERR_DCERPC);
        CASE_CODE (SC_ERR_DETECT_PREPARE);
        CASE_CODE (SC_ERR_AHO_CORASICK);
        CASE_CODE (SC_ERR_REFERENCE_CONFIG);
        CASE_CODE (SC_ERR_DUPLICATE_SIG);
        CASE_CODE (SC_WARN_PCAP_MULTI_DEV_EXPERIMENTAL);
        CASE_CODE (SC_ERR_PCAP_MULTI_DEV_NO_SUPPORT);
        CASE_CODE (SC_ERR_HTTP_METHOD_NEEDS_PRECEEDING_CONTENT);
        CASE_CODE (SC_ERR_HTTP_METHOD_INCOMPATIBLE_WITH_RAWBYTES);
        CASE_CODE (SC_ERR_HTTP_METHOD_RELATIVE_MISSING);
        CASE_CODE (SC_ERR_HTTP_COOKIE_NEEDS_PRECEEDING_CONTENT);
        CASE_CODE (SC_ERR_HTTP_COOKIE_INCOMPATIBLE_WITH_RAWBYTES);
        CASE_CODE (SC_ERR_HTTP_COOKIE_RELATIVE_MISSING);
        CASE_CODE (SC_ERR_LOGPCAP_SGUIL_BASE_DIR_MISSING);
        CASE_CODE (SC_ERR_UNKNOWN_DECODE_EVENT);
        CASE_CODE (SC_ERR_RUNMODE);
        CASE_CODE (SC_ERR_SHUTDOWN);
        CASE_CODE (SC_ERR_INVALID_DIRECTION);
        CASE_CODE (SC_ERR_AFP_CREATE);
        CASE_CODE (SC_ERR_AFP_READ);
        CASE_CODE (SC_ERR_AFP_DISPATCH);
        CASE_CODE (SC_ERR_CMD_LINE);
        CASE_CODE (SC_ERR_SIZE_PARSE);
        CASE_CODE (SC_ERR_RAWBYTES_FILE_DATA);
        CASE_CODE (SC_ERR_SOCKET);
        CASE_CODE (SC_ERR_PCAP_TRANSLATE);
        CASE_CODE (SC_WARN_OUTDATED_LIBHTP);
        CASE_CODE (SC_WARN_DEPRECATED);
        CASE_CODE (SC_WARN_PROFILE);
        CASE_CODE (SC_ERR_FLOW_INIT);
        CASE_CODE (SC_ERR_HOST_INIT);
        CASE_CODE (SC_ERR_MEM_BUFFER_API);
        CASE_CODE (SC_ERR_INVALID_MD5);
        CASE_CODE (SC_ERR_NO_MD5_SUPPORT);
        CASE_CODE (SC_ERR_EVENT_ENGINE);
        CASE_CODE (SC_ERR_NO_LUAJIT_SUPPORT);
        CASE_CODE (SC_ERR_LUAJIT_ERROR);
        CASE_CODE (SC_ERR_NO_GEOIP_SUPPORT);
        CASE_CODE (SC_ERR_GEOIP_ERROR);
        CASE_CODE (SC_ERR_DEFRAG_INIT);
        CASE_CODE (SC_ERR_NO_REPUTATION);
        CASE_CODE (SC_ERR_NOT_SUPPORTED);
        CASE_CODE (SC_ERR_LIVE_RULE_SWAP);
        CASE_CODE (SC_ERR_CUDA_BUFFER_ERROR);
        default:
            return "UNKNOWN_ERROR";
    }
}
