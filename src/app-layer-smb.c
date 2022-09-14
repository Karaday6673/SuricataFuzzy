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

#include "suricata-common.h"
#include "suricata.h"

#include "app-layer-protos.h"
#include "app-layer-detect-proto.h"
#include "app-layer-parser.h"

#include "util-unittest.h"

#include "rust.h"
#include "app-layer-smb.h"
#include "util-misc.h"

#include "app-layer-htp-file.h"
#include "app-layer-htp-range.h"
#include "util-print.h"

static StreamingBufferConfig sbcfg = STREAMING_BUFFER_CONFIG_INITIALIZER;
static SuricataFileContext sfc = { &sbcfg };

#ifdef UNITTESTS
static void SMBParserRegisterTests(void);
#endif

#define SMB_URL_PREFIX_LEN 6
#define MAX_ADDR_LEN       46
#define GUID_LEN           16

static size_t SmbSetKey(const Flow *f, const uint8_t *guid, uint8_t *hkey)
{
    memcpy(hkey, "smb://", SMB_URL_PREFIX_LEN);
    int printIp = FLOW_IS_IPV4(f) ? AF_INET : AF_INET6;
    PrintInet(printIp, (const void *)&(f->src.address), (char *)(hkey + SMB_URL_PREFIX_LEN),
            2 * GUID_LEN + MAX_ADDR_LEN + 1);
    size_t key_len = strlen((const char *)hkey);
    hkey[key_len] = '/';
    key_len++;
    rs_to_hex(hkey + key_len, sizeof(hkey) - key_len, guid, GUID_LEN);
    key_len += 2 * GUID_LEN;
    return key_len;
}

void SmbMultiSetFileSize(const Flow *f, const uint8_t *guid, uint64_t eof, const uint8_t *filename,
        uint16_t name_len, const StreamingBufferConfig *files_sbcfg)
{
    uint8_t hkey[2 * GUID_LEN + MAX_ADDR_LEN + 1 + SMB_URL_PREFIX_LEN] = { 0 };
    size_t keylen = SmbSetKey(f, guid, hkey);
    uint16_t flags = FileFlowToFlags(f, STREAM_TOSERVER);

    HttpRangeContainerFile *file_range_container = HttpRangeContainerUrlGet(hkey, keylen, f);
    file_range_container->totalsize = eof;
    if (file_range_container->files != NULL && file_range_container->files->tail == NULL) {
        if (FileOpenFileWithId(file_range_container->files, files_sbcfg, 0, filename, name_len,
                    NULL, 0, flags) != 0) {
            SCLogDebug("open file for range failed");
        }
    } else {
        FileSetName(file_range_container->files->tail, filename, name_len);
    }
    THashDecrUsecnt(file_range_container->hdata);
    THashDataUnlock(file_range_container->hdata);
}

HttpRangeContainerBlock *SmbMultiStartFileChunk(const Flow *f, const uint8_t *guid, uint16_t flags,
        FileContainer *fc, const StreamingBufferConfig *files_sbcfg, bool *added, uint64_t offset,
        uint32_t rlen, const uint8_t *data, uint32_t data_len)
{
    HttpRangeContainerBlock *r = NULL;

    FileContentRange fcr;
    if (offset > INT64_MAX || offset + rlen > INT64_MAX) {
        return NULL;
    }
    fcr.start = offset;
    // total size is set by SmbMultiSetFileSize
    fcr.size = 0;
    fcr.end = offset + rlen;

    uint8_t hkey[2 * GUID_LEN + MAX_ADDR_LEN + 1 + SMB_URL_PREFIX_LEN] = { 0 };
    size_t key_len = SmbSetKey(f, guid, hkey);

    r = HttpRangeContainerOpenFile(
            hkey, key_len, f, &fcr, files_sbcfg, NULL, 0, flags, data, data_len);
    if (r) {
        if (data_len >= rlen) {
            *added = HTPFileCloseHandleRange(fc, flags, r, NULL, 0);
            HttpRangeFreeBlock(r);
            r = NULL;
        }
    }
    return r;
}

void RegisterSMBParsers(void)
{
    rs_smb_init(&sfc);
    rs_smb_register_parser();

#ifdef UNITTESTS
    AppLayerParserRegisterProtocolUnittests(IPPROTO_TCP, ALPROTO_SMB, SMBParserRegisterTests);
#endif

    return;
}

#ifdef UNITTESTS
#include "stream-tcp.h"
#include "util-unittest-helper.h"

/** \test multi transactions and cleanup */
static int SMBParserTxCleanupTest(void)
{
    uint64_t ret[4];
    AppLayerParserThreadCtx *alp_tctx = AppLayerParserThreadCtxAlloc();
    FAIL_IF_NULL(alp_tctx);

    StreamTcpInitConfig(true);
    TcpSession ssn;
    memset(&ssn, 0, sizeof(ssn));

    Flow *f = UTHBuildFlow(AF_INET, "1.2.3.4", "1.2.3.5", 1024, 445);
    FAIL_IF_NULL(f);
    f->protoctx = &ssn;
    f->proto = IPPROTO_TCP;
    f->alproto = ALPROTO_SMB;

    char req_str[] ="\x00\x00\x00\x79\xfe\x53\x4d\x42\x40\x00\x01\x00\x00\x00\x00\x00" \
                     "\x05\x00\xe0\x1e\x10\x00\x00\x00\x00\x00\x00\x00\x0b\x00\x00\x00" \
                     "\x00\x00\x00\x00\x00\x00\x00\x00\x10\x72\xd2\x9f\x36\xc2\x08\x14" \
                     "\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00" \
                     "\x00\x00\x00\x00\x39\x00\x00\x00\x02\x00\x00\x00\x00\x00\x00\x00" \
                     "\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x80\x00\x00\x00" \
                     "\x00\x00\x00\x00\x07\x00\x00\x00\x01\x00\x00\x00\x00\x00\x00\x00" \
                     "\x78\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00";
    req_str[28] = 0x01;
    int r = AppLayerParserParse(NULL, alp_tctx, f, ALPROTO_SMB,
                                STREAM_TOSERVER | STREAM_START, (uint8_t *)req_str, sizeof(req_str));
    FAIL_IF_NOT(r == 0);
    req_str[28]++;
    r = AppLayerParserParse(NULL, alp_tctx, f, ALPROTO_SMB,
                                STREAM_TOSERVER, (uint8_t *)req_str, sizeof(req_str));
    FAIL_IF_NOT(r == 0);
    req_str[28]++;
    r = AppLayerParserParse(NULL, alp_tctx, f, ALPROTO_SMB,
                                STREAM_TOSERVER, (uint8_t *)req_str, sizeof(req_str));
    FAIL_IF_NOT(r == 0);
    req_str[28]++;
    r = AppLayerParserParse(NULL, alp_tctx, f, ALPROTO_SMB,
                                STREAM_TOSERVER, (uint8_t *)req_str, sizeof(req_str));
    FAIL_IF_NOT(r == 0);
    req_str[28]++;
    r = AppLayerParserParse(NULL, alp_tctx, f, ALPROTO_SMB,
                                STREAM_TOSERVER, (uint8_t *)req_str, sizeof(req_str));
    FAIL_IF_NOT(r == 0);
    req_str[28]++;
    r = AppLayerParserParse(NULL, alp_tctx, f, ALPROTO_SMB,
                                STREAM_TOSERVER, (uint8_t *)req_str, sizeof(req_str));
    FAIL_IF_NOT(r == 0);
    req_str[28]++;
    r = AppLayerParserParse(NULL, alp_tctx, f, ALPROTO_SMB,
                                STREAM_TOSERVER, (uint8_t *)req_str, sizeof(req_str));
    FAIL_IF_NOT(r == 0);
    req_str[28]++;
    r = AppLayerParserParse(NULL, alp_tctx, f, ALPROTO_SMB,
                                STREAM_TOSERVER, (uint8_t *)req_str, sizeof(req_str));
    FAIL_IF_NOT(r == 0);
    req_str[28]++;

    AppLayerParserTransactionsCleanup(f);
    UTHAppLayerParserStateGetIds(f->alparser, &ret[0], &ret[1], &ret[2], &ret[3]);
    FAIL_IF_NOT(ret[0] == 0); // inspect_id[0]
    FAIL_IF_NOT(ret[1] == 0); // inspect_id[1]
    FAIL_IF_NOT(ret[2] == 0); // log_id
    FAIL_IF_NOT(ret[3] == 0); // min_id

    char resp_str[] = "\x00\x00\x00\x98\xfe\x53\x4d\x42\x40\x00\x01\x00\x00\x00\x00\x00" \
                       "\x05\x00\x21\x00\x11\x00\x00\x00\x00\x00\x00\x00\x0b\x00\x00\x00" \
                       "\x00\x00\x00\x00\x00\x00\x00\x00\x10\x72\xd2\x9f\x36\xc2\x08\x14" \
                       "\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00" \
                       "\x00\x00\x00\x00\x59\x00\x00\x00\x01\x00\x00\x00\x48\x38\x40\xb3" \
                       "\x0f\xa8\xd3\x01\x84\x9a\x2b\x46\xf7\xa8\xd3\x01\x48\x38\x40\xb3" \
                       "\x0f\xa8\xd3\x01\x48\x38\x40\xb3\x0f\xa8\xd3\x01\x00\x00\x00\x00" \
                       "\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x10\x00\x00\x00" \
                       "\x00\x00\x00\x00\x9e\x8f\xb8\x91\x00\x00\x00\x00\x01\x5b\x11\xbb" \
                       "\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00";

    resp_str[28] = 0x01;
    r = AppLayerParserParse(NULL, alp_tctx, f, ALPROTO_SMB,
                                STREAM_TOCLIENT | STREAM_START, (uint8_t *)resp_str, sizeof(resp_str));
    FAIL_IF_NOT(r == 0);
    resp_str[28] = 0x04;
    r = AppLayerParserParse(NULL, alp_tctx, f, ALPROTO_SMB,
                                STREAM_TOCLIENT, (uint8_t *)resp_str, sizeof(resp_str));
    FAIL_IF_NOT(r == 0);
    resp_str[28] = 0x05;
    r = AppLayerParserParse(NULL, alp_tctx, f, ALPROTO_SMB,
                                STREAM_TOCLIENT, (uint8_t *)resp_str, sizeof(resp_str));
    FAIL_IF_NOT(r == 0);
    resp_str[28] = 0x06;
    r = AppLayerParserParse(NULL, alp_tctx, f, ALPROTO_SMB,
                                STREAM_TOCLIENT, (uint8_t *)resp_str, sizeof(resp_str));
    FAIL_IF_NOT(r == 0);
    resp_str[28] = 0x08;
    r = AppLayerParserParse(NULL, alp_tctx, f, ALPROTO_SMB,
                                STREAM_TOCLIENT, (uint8_t *)resp_str, sizeof(resp_str));
    FAIL_IF_NOT(r == 0);
    resp_str[28] = 0x02;
    r = AppLayerParserParse(NULL, alp_tctx, f, ALPROTO_SMB,
                                STREAM_TOCLIENT, (uint8_t *)resp_str, sizeof(resp_str));
    FAIL_IF_NOT(r == 0);
    resp_str[28] = 0x07;
    r = AppLayerParserParse(NULL, alp_tctx, f, ALPROTO_SMB,
                                STREAM_TOCLIENT, (uint8_t *)resp_str, sizeof(resp_str));
    FAIL_IF_NOT(r == 0);
    AppLayerParserTransactionsCleanup(f);

    UTHAppLayerParserStateGetIds(f->alparser, &ret[0], &ret[1], &ret[2], &ret[3]);
    FAIL_IF_NOT(ret[0] == 2); // inspect_id[0]
    FAIL_IF_NOT(ret[1] == 2); // inspect_id[1]
    FAIL_IF_NOT(ret[2] == 2); // log_id
    FAIL_IF_NOT(ret[3] == 2); // min_id

    resp_str[28] = 0x03;
    r = AppLayerParserParse(NULL, alp_tctx, f, ALPROTO_SMB,
                                STREAM_TOCLIENT, (uint8_t *)resp_str, sizeof(resp_str));
    FAIL_IF_NOT(r == 0);
    AppLayerParserTransactionsCleanup(f);

    UTHAppLayerParserStateGetIds(f->alparser, &ret[0], &ret[1], &ret[2], &ret[3]);
    FAIL_IF_NOT(ret[0] == 8); // inspect_id[0]
    FAIL_IF_NOT(ret[1] == 8); // inspect_id[1]
    FAIL_IF_NOT(ret[2] == 8); // log_id
    FAIL_IF_NOT(ret[3] == 8); // min_id

    req_str[28] = 0x09;
    r = AppLayerParserParse(NULL, alp_tctx, f, ALPROTO_SMB,
                                STREAM_TOSERVER | STREAM_EOF, (uint8_t *)req_str, sizeof(req_str));
    FAIL_IF_NOT(r == 0);
    AppLayerParserTransactionsCleanup(f);

    UTHAppLayerParserStateGetIds(f->alparser, &ret[0], &ret[1], &ret[2], &ret[3]);
    FAIL_IF_NOT(ret[0] == 8); // inspect_id[0] not updated by ..Cleanup() until full tx is done
    FAIL_IF_NOT(ret[1] == 8); // inspect_id[1]
    FAIL_IF_NOT(ret[2] == 8); // log_id
    FAIL_IF_NOT(ret[3] == 8); // min_id

    resp_str[28] = 0x09;
    r = AppLayerParserParse(NULL, alp_tctx, f, ALPROTO_SMB,
                                STREAM_TOCLIENT | STREAM_EOF, (uint8_t *)resp_str, sizeof(resp_str));
    FAIL_IF_NOT(r == 0);
    AppLayerParserTransactionsCleanup(f);

    UTHAppLayerParserStateGetIds(f->alparser, &ret[0], &ret[1], &ret[2], &ret[3]);
    FAIL_IF_NOT(ret[0] == 9); // inspect_id[0]
    FAIL_IF_NOT(ret[1] == 9); // inspect_id[1]
    FAIL_IF_NOT(ret[2] == 9); // log_id
    FAIL_IF_NOT(ret[3] == 9); // min_id

    AppLayerParserThreadCtxFree(alp_tctx);
    StreamTcpFreeConfig(true);
    UTHFreeFlow(f);

    PASS;
}

static void SMBParserRegisterTests(void)
{
    UtRegisterTest("SMBParserTxCleanupTest", SMBParserTxCleanupTest);
}

#endif /* UNITTESTS */
