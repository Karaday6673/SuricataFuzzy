/* Copyright (C) 2007-2021 Open Information Security Foundation
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
        CASE_CODE (SC_ERR_PCRE_COPY_SUBSTRING);
        CASE_CODE(SC_ERR_PCRE_COMPILE);
        CASE_CODE(SC_ERR_PCRE_PARSE);
        CASE_CODE (SC_ERR_LOG_FG_FILTER_MATCH);
        CASE_CODE (SC_ERR_PCAP_DISPATCH);
        CASE_CODE (SC_ERR_PCAP_CREATE);
        CASE_CODE (SC_ERR_PCAP_SET_SNAPLEN);
        CASE_CODE (SC_ERR_PCAP_SET_PROMISC);
        CASE_CODE(SC_ERR_PCAP_SET_TIMEOUT);
        CASE_CODE (SC_ERR_PCAP_OPEN_OFFLINE);
        CASE_CODE (SC_ERR_PCAP_ACTIVATE_HANDLE);
        CASE_CODE (SC_ERR_PCAP_SET_BUFF_SIZE);
        CASE_CODE (SC_ERR_NO_PCAP_SET_BUFFER_SIZE);
        CASE_CODE (SC_ERR_NO_PF_RING);
        CASE_CODE(SC_ERR_PF_RING_RECV);
        CASE_CODE (SC_ERR_PF_RING_OPEN);
        CASE_CODE (SC_ERR_GET_CLUSTER_TYPE_FAILED);
        CASE_CODE (SC_ERR_INVALID_CLUSTER_TYPE);
        CASE_CODE (SC_ERR_PF_RING_SET_CLUSTER_FAILED);
        CASE_CODE (SC_ERR_DATALINK_UNIMPLEMENTED);
        CASE_CODE (SC_ERR_INVALID_SIGNATURE);
        CASE_CODE (SC_ERR_OPENING_FILE);
        CASE_CODE (SC_ERR_OPENING_RULE_FILE);
        CASE_CODE (SC_ERR_NO_RULES);
        CASE_CODE(SC_ERR_NO_RULES_LOADED);
        CASE_CODE (SC_ERR_INVALID_CHECKSUM);
        CASE_CODE (SC_ERR_SPRINTF);
        CASE_CODE (SC_ERR_FATAL);
        CASE_CODE(SC_ERR_INVALID_ARGUMENT);
        CASE_CODE (SC_ERR_INVALID_ENUM_MAP);
        CASE_CODE (SC_ERR_INVALID_IP_NETBLOCK);
        CASE_CODE (SC_ERR_INVALID_IPV4_ADDR);
        CASE_CODE (SC_ERR_INVALID_IPV6_ADDR);
        CASE_CODE (SC_ERR_INVALID_RUNMODE);
        CASE_CODE (SC_ERR_COMPLETE_PORT_SPACE_NEGATED);
        CASE_CODE (SC_ERR_NO_PORTS_LEFT_AFTER_MERGE);
        CASE_CODE (SC_ERR_NEGATED_VALUE_IN_PORT_RANGE);
        CASE_CODE(SC_ERR_PORT_PARSE_INSERT_STRING);
        CASE_CODE (SC_ERR_INVALID_NUMERIC_VALUE);
        CASE_CODE(SC_ERR_NUMERIC_VALUE_ERANGE);
        CASE_CODE (SC_ERR_ARG_LEN_LONG);
        CASE_CODE(SC_ERR_ALPARSER);
        CASE_CODE (SC_ERR_REASSEMBLY);
        CASE_CODE (SC_ERR_POOL_INIT);
        CASE_CODE (SC_ERR_UNIMPLEMENTED);
        CASE_CODE (SC_ERR_ADDRESS_ENGINE_GENERIC);
        CASE_CODE(SC_ERR_PORT_ENGINE_GENERIC);
        CASE_CODE(SC_ERR_IPONLY_RADIX);
        CASE_CODE(SC_ERR_HTTP_LOG_GENERIC);
        CASE_CODE (SC_ERR_FWRITE);
        CASE_CODE (SC_ERR_FOPEN);
        CASE_CODE (SC_ERR_THREAD_NICE_PRIO);
        CASE_CODE (SC_ERR_THREAD_SPAWN);
        CASE_CODE (SC_ERR_THREAD_CREATE);
        CASE_CODE (SC_ERR_THREAD_INIT);
        CASE_CODE(SC_ERR_THREAD_DEINIT);
        CASE_CODE (SC_ERR_UNDEFINED_VAR);
        CASE_CODE (SC_ERR_RULE_KEYWORD_UNKNOWN);
        CASE_CODE(SC_ERR_FLAGS_MODIFIER);
        CASE_CODE (SC_ERR_WITHIN_MISSING_CONTENT);
        CASE_CODE (SC_ERR_WITHIN_INVALID);
        CASE_CODE (SC_ERR_DEPTH_MISSING_CONTENT);
        CASE_CODE (SC_ERR_OFFSET_MISSING_CONTENT);
        CASE_CODE (SC_ERR_NOCASE_MISSING_PATTERN);
        CASE_CODE(SC_ERR_RAWBYTES_MISSING_CONTENT);
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
        CASE_CODE(SC_ERR_IPFW_SETSOCKOPT);
        CASE_CODE (SC_WARN_IPFW_RECV);
        CASE_CODE(SC_WARN_IPFW_XMIT);
        CASE_CODE (SC_WARN_IPFW_UNBIND);
        CASE_CODE (SC_ERR_MULTIPLE_RUN_MODE);
        CASE_CODE (SC_ERR_BPF);
        CASE_CODE (SC_ERR_MISSING_CONFIG_PARAM);
        CASE_CODE (SC_ERR_UNKNOWN_VALUE);
        CASE_CODE (SC_ERR_INVALID_VALUE);
        CASE_CODE (SC_ERR_UNKNOWN_REGEX_MOD);
        CASE_CODE(SC_ERR_INVALID_OPERATOR);
        CASE_CODE(SC_ERR_NFQ_NOSUPPORT);
        CASE_CODE (SC_ERR_NFQ_HANDLE_PKT);
        CASE_CODE (SC_ERR_NFLOG_NOSUPPORT);
        CASE_CODE(SC_ERR_NFLOG_OPEN);
        CASE_CODE (SC_ERR_NFLOG_MAX_BUFSIZ);
        CASE_CODE (SC_ERR_NFLOG_SET_MODE);
        CASE_CODE(SC_ERR_NFLOG_HANDLE_PKT);
        CASE_CODE (SC_ERR_NFLOG_FD);
        CASE_CODE (SC_WARN_NFLOG_SETSOCKOPT);
        CASE_CODE (SC_WARN_NFLOG_RECV);
        CASE_CODE (SC_WARN_NFLOG_LOSING_EVENTS);
        CASE_CODE(SC_WARN_NFLOG_MAXBUFSIZ_REACHED);
        CASE_CODE (SC_ERR_TM_THREADS_ERROR);
        CASE_CODE(SC_ERR_TM_MODULES_ERROR);
        CASE_CODE(SC_ERR_INVALID_YAML_CONF_ENTRY);
        CASE_CODE (SC_ERR_CONFLICTING_RULE_KEYWORDS);
        CASE_CODE (SC_ERR_INITIALIZATION);
        CASE_CODE (SC_ERR_INVALID_ACTION);
        CASE_CODE (SC_ERR_LIBNET_REQUIRED_FOR_ACTION);
        CASE_CODE(SC_ERR_LIBNET_INIT);
        CASE_CODE (SC_ERR_LIBNET_BUILD_FAILED);
        CASE_CODE (SC_ERR_LIBNET_WRITE_FAILED);
        CASE_CODE(SC_ERR_LIBNET_NOT_ENABLED);
        CASE_CODE (SC_ERR_REFERENCE_UNKNOWN);
        CASE_CODE (SC_ERR_PIDFILE_SNPRINTF);
        CASE_CODE (SC_ERR_PIDFILE_OPEN);
        CASE_CODE (SC_ERR_PIDFILE_WRITE);
        CASE_CODE (SC_ERR_PIDFILE_DAEMON);
        CASE_CODE (SC_ERR_UID_FAILED);
        CASE_CODE(SC_ERR_GID_FAILED);
        CASE_CODE (SC_ERR_LIBCAP_NG_REQUIRED);
        CASE_CODE (SC_ERR_LIBNET11_INCOMPATIBLE_WITH_LIBCAP_NG);
        CASE_CODE (SC_ERR_PLEDGE_FAILED);
        CASE_CODE (SC_WARN_FLOW_EMERGENCY);
        CASE_CODE (SC_ERR_SVC);
        CASE_CODE (SC_ERR_ERF_DAG_OPEN_FAILED);
        CASE_CODE (SC_ERR_ERF_DAG_STREAM_OPEN_FAILED);
        CASE_CODE (SC_ERR_ERF_DAG_STREAM_START_FAILED);
        CASE_CODE (SC_ERR_ERF_DAG_STREAM_SET_FAILED);
        CASE_CODE(SC_ERR_ERF_DAG_STREAM_READ_FAILED);
        CASE_CODE(SC_ERR_NAPATECH_OPEN_FAILED);
        CASE_CODE (SC_ERR_NAPATECH_NOSUPPORT);
        CASE_CODE (SC_ERR_NAPATECH_REQUIRED);
        CASE_CODE (SC_ERR_NAPATECH_TIMESTAMP_TYPE_NOT_SUPPORTED);
        CASE_CODE (SC_ERR_NAPATECH_INIT_FAILED);
        CASE_CODE (SC_ERR_NAPATECH_CONFIG_STREAM);
        CASE_CODE(SC_ERR_NAPATECH_STREAMS_REGISTER_FAILED);
        CASE_CODE (SC_ERR_NAPATECH_PARSE_CONFIG);
        CASE_CODE(SC_WARN_COMPATIBILITY);
        CASE_CODE (SC_ERR_DETECT_PREPARE);
        CASE_CODE (SC_ERR_AHO_CORASICK);
        CASE_CODE (SC_ERR_REFERENCE_CONFIG);
        CASE_CODE(SC_ERR_DUPLICATE_SIG);
        CASE_CODE(SC_ERR_PCAP_MULTI_DEV_NO_SUPPORT);
        CASE_CODE (SC_ERR_UNKNOWN_DECODE_EVENT);
        CASE_CODE (SC_ERR_RUNMODE);
        CASE_CODE (SC_ERR_SHUTDOWN);
        CASE_CODE (SC_ERR_INVALID_DIRECTION);
        CASE_CODE (SC_ERR_AFP_CREATE);
        CASE_CODE(SC_ERR_AFP_READ);
        CASE_CODE (SC_ERR_CMD_LINE);
        CASE_CODE (SC_ERR_SIZE_PARSE);
        CASE_CODE (SC_ERR_RAWBYTES_BUFFER);
        CASE_CODE (SC_ERR_SOCKET);
        CASE_CODE (SC_ERR_PCAP_TRANSLATE);
        CASE_CODE (SC_WARN_OUTDATED_LIBHTP);
        CASE_CODE (SC_WARN_DEPRECATED);
        CASE_CODE (SC_WARN_PROFILE);
        CASE_CODE (SC_ERR_FLOW_INIT);
        CASE_CODE (SC_ERR_HOST_INIT);
        CASE_CODE(SC_ERR_MEM_BUFFER_API);
        CASE_CODE (SC_ERR_EVENT_ENGINE);
        CASE_CODE (SC_ERR_NO_LUA_SUPPORT);
        CASE_CODE (SC_ERR_LUA_ERROR);
        CASE_CODE(SC_ERR_NO_GEOIP_SUPPORT);
        CASE_CODE (SC_ERR_DEFRAG_INIT);
        CASE_CODE (SC_ERR_NO_REPUTATION);
        CASE_CODE (SC_ERR_NOT_SUPPORTED);
        CASE_CODE (SC_ERR_LIVE_RULE_SWAP);
        CASE_CODE (SC_WARN_UNCOMMON);
        CASE_CODE (SC_ERR_SYSCALL);
        CASE_CODE (SC_ERR_SYSCONF);
        CASE_CODE (SC_ERR_INVALID_ARGUMENTS);
        CASE_CODE (SC_ERR_STATS_NOT_INIT);
        CASE_CODE (SC_ERR_NFQ_OPEN);
        CASE_CODE (SC_ERR_NFQ_MAXLEN);
        CASE_CODE (SC_ERR_NFQ_CREATE_QUEUE);
        CASE_CODE (SC_ERR_NFQ_SET_MODE);
        CASE_CODE (SC_ERR_NFQ_SETSOCKOPT);
        CASE_CODE (SC_ERR_NFQ_RECV);
        CASE_CODE (SC_ERR_NFQ_SET_VERDICT);
        CASE_CODE (SC_ERR_NFQ_THREAD_INIT);
        CASE_CODE (SC_ERR_DAEMON);
        CASE_CODE(SC_ERR_TLS_LOG_GENERIC);
        CASE_CODE (SC_ERR_DAG_REQUIRED);
        CASE_CODE (SC_ERR_DAG_NOSUPPORT);
        CASE_CODE (SC_ERR_NO_AF_PACKET);
        CASE_CODE (SC_ERR_PCAP_FILE_DELETE_FAILED);
        CASE_CODE (SC_ERR_MAGIC_OPEN);
        CASE_CODE(SC_ERR_MAGIC_LOAD);
        CASE_CODE (SC_WARN_OPTION_OBSOLETE);
        CASE_CODE (SC_WARN_NO_UNITTESTS);
        CASE_CODE (SC_ERR_THREAD_QUEUE);
        CASE_CODE (SC_WARN_XFF_INVALID_MODE);
        CASE_CODE (SC_WARN_XFF_INVALID_HEADER);
        CASE_CODE(SC_WARN_XFF_INVALID_DEPLOYMENT);
        CASE_CODE(SC_ERR_CONF_YAML_ERROR);
        CASE_CODE(SC_ERR_CONF_NAME_TOO_LONG);
        CASE_CODE (SC_WARN_LUA_SCRIPT);
        CASE_CODE (SC_ERR_LUA_SCRIPT);
        CASE_CODE (SC_WARN_NO_STATS_LOGGERS);
        CASE_CODE (SC_ERR_NO_NETMAP);
        CASE_CODE (SC_ERR_NETMAP_CREATE);
        CASE_CODE (SC_ERR_NETMAP_READ);
        CASE_CODE (SC_ERR_IPPAIR_INIT);
        CASE_CODE (SC_ERR_MT_NO_SELECTOR);
        CASE_CODE (SC_ERR_MT_DUPLICATE_TENANT);
        CASE_CODE(SC_ERR_MT_NO_MAPPING);
        CASE_CODE (SC_ERR_INVALID_RULE_ARGUMENT);
        CASE_CODE (SC_ERR_STATS_LOG_NEGATED);
        CASE_CODE (SC_ERR_JSON_STATS_LOG_NEGATED);
        CASE_CODE (SC_ERR_DEPRECATED_CONF);
        CASE_CODE (SC_WARN_FASTER_CAPTURE_AVAILABLE);
        CASE_CODE (SC_WARN_POOR_RULE);
        CASE_CODE (SC_ERR_ALERT_PAYLOAD_BUFFER);
        CASE_CODE (SC_ERR_STATS_LOG_GENERIC);
        CASE_CODE(SC_ERR_TCPDATA_LOG_GENERIC);
        CASE_CODE (SC_ERR_NIC_OFFLOADING);
        CASE_CODE (SC_ERR_NO_FILES_FOR_PROTOCOL);
        CASE_CODE(SC_ERR_INVALID_HASH);
        CASE_CODE (SC_ERR_DIR_OPEN);
        CASE_CODE(SC_WARN_REMOVE_FILE);
        CASE_CODE (SC_WARN_DUPLICATE_OUTPUT);
        CASE_CODE (SC_ERR_NO_MAGIC_SUPPORT);
        CASE_CODE (SC_ERR_VAR_LIMIT);
        CASE_CODE (SC_WARN_CHMOD);
        CASE_CODE (SC_WARN_LOG_CF_TOO_MANY_NODES);
        CASE_CODE (SC_WARN_EVENT_DROPPED);
        CASE_CODE(SC_ERR_NO_REDIS_ASYNC);
        CASE_CODE (SC_ERR_BYPASS_NOT_SUPPORTED);
        CASE_CODE (SC_WARN_RENAMING_FILE);
        CASE_CODE (SC_ERR_PF_RING_VLAN);
        CASE_CODE (SC_ERR_CREATE_DIRECTORY);
        CASE_CODE(SC_WARN_FLOWBIT);
        CASE_CODE (SC_WARN_NO_JA3_SUPPORT);
        CASE_CODE (SC_WARN_JA3_DISABLED);
        CASE_CODE (SC_ERR_PCAP_LOG_COMPRESS);
        CASE_CODE (SC_ERR_FSEEK);
        CASE_CODE (SC_ERR_WINDIVERT_GENERIC);
        CASE_CODE (SC_ERR_WINDIVERT_NOSUPPORT);
        CASE_CODE (SC_ERR_WINDIVERT_INVALID_FILTER);
        CASE_CODE(SC_ERR_WINDIVERT_TOOLONG_FILTER);
        CASE_CODE (SC_WARN_EVE_MISSING_EVENTS);
        CASE_CODE (SC_ERR_THASH_INIT);
        CASE_CODE (SC_ERR_DATASET);
        CASE_CODE (SC_WARN_ANOMALY_CONFIG);
        CASE_CODE(SC_WARN_ALERT_CONFIG);
        CASE_CODE (SC_ERR_ERF_BAD_RLEN);
        CASE_CODE (SC_WARN_ERSPAN_CONFIG);
        CASE_CODE (SC_WARN_HASSH_DISABLED);
        CASE_CODE(SC_WARN_FILESTORE_CONFIG);
        CASE_CODE (SC_ERR_PLUGIN);
        CASE_CODE(SC_ERR_LOG_OUTPUT);
        CASE_CODE(SC_ERR_RULE_INVALID_UTF8);
        CASE_CODE(SC_ERR_HASHING_DISABLED);
        CASE_CODE(SC_WARN_THRESH_CONFIG);
        CASE_CODE(SC_ERR_NO_DPDK);
        CASE_CODE(SC_ERR_DPDK_INIT);
        CASE_CODE(SC_ERR_DPDK_EAL_INIT);
        CASE_CODE(SC_ERR_DPDK_EAL_DEINIT);
        CASE_CODE(SC_ERR_DPDK_CONF);
        CASE_CODE(SC_WARN_DPDK_CONF);
        CASE_CODE(SC_ERR_SIGNAL);
        CASE_CODE(SC_WARN_CHOWN);
        CASE_CODE(SC_ERR_HASH_ADD);
        CASE_CODE(SC_WARN_CLASSIFICATION_CONFIG);

        CASE_CODE (SC_ERR_MAX);
    }

    return "UNKNOWN_ERROR";
}
