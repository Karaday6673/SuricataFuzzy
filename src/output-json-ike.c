/* Copyright (C) 2018-2020 Open Information Security Foundation
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
 * \author Pierre Chifflier <chifflier@wzdftpd.net>
 *
 * Implement JSON/eve logging app-layer IKE.
 */

#include "suricata-common.h"
#include "debug.h"
#include "detect.h"
#include "pkt-var.h"
#include "conf.h"

#include "threads.h"
#include "threadvars.h"
#include "tm-threads.h"

#include "util-unittest.h"
#include "util-buffer.h"
#include "util-debug.h"
#include "util-byte.h"

#include "output.h"
#include "output-json.h"

#include "app-layer.h"
#include "app-layer-parser.h"

#include "app-layer-ike.h"
#include "output-json-ike.h"

#include "rust.h"

typedef struct LogIKEFileCtx_ {
    LogFileCtx *file_ctx;
    OutputJsonCommonSettings cfg;
} LogIKEFileCtx;

typedef struct LogIKELogThread_ {
    LogFileCtx *file_ctx;
    LogIKEFileCtx *ikelog_ctx;
    MemBuffer *buffer;
} LogIKELogThread;

static int JsonIKELogger(ThreadVars *tv, void *thread_data, const Packet *p, Flow *f, void *state,
        void *tx, uint64_t tx_id)
{
    IKETransaction *iketx = tx;
    LogIKELogThread *thread = thread_data;

    JsonBuilder *jb = CreateEveHeader((Packet *)p, LOG_DIR_PACKET, "ike", NULL);
    if (unlikely(jb == NULL)) {
        return TM_ECODE_FAILED;
    }

    EveAddCommonOptions(&thread->ikelog_ctx->cfg, p, f, jb);

    jb_open_object(jb, "ike");
    if (unlikely(!rs_ike_log_json_response(state, iketx, jb))) {
        goto error;
    }
    jb_close(jb);

    MemBufferReset(thread->buffer);
    OutputJsonBuilderBuffer(jb, thread->file_ctx, &thread->buffer);

    jb_free(jb);
    return TM_ECODE_OK;

error:
    jb_free(jb);
    return TM_ECODE_FAILED;
}

static void OutputIKELogDeInitCtxSub(OutputCtx *output_ctx)
{
    LogIKEFileCtx *ikelog_ctx = (LogIKEFileCtx *)output_ctx->data;
    SCFree(ikelog_ctx);
    SCFree(output_ctx);
}

static OutputInitResult OutputIKELogInitSub(ConfNode *conf, OutputCtx *parent_ctx)
{
    OutputInitResult result = { NULL, false };
    OutputJsonCtx *ajt = parent_ctx->data;

    LogIKEFileCtx *ikelog_ctx = SCCalloc(1, sizeof(*ikelog_ctx));
    if (unlikely(ikelog_ctx == NULL)) {
        return result;
    }
    ikelog_ctx->file_ctx = ajt->file_ctx;
    ikelog_ctx->cfg = ajt->cfg;

    OutputCtx *output_ctx = SCCalloc(1, sizeof(*output_ctx));
    if (unlikely(output_ctx == NULL)) {
        SCFree(ikelog_ctx);
        return result;
    }
    output_ctx->data = ikelog_ctx;
    output_ctx->DeInit = OutputIKELogDeInitCtxSub;

    SCLogDebug("IKE log sub-module initialized.");

    AppLayerParserRegisterLogger(IPPROTO_UDP, ALPROTO_IKE);

    result.ctx = output_ctx;
    result.ok = true;
    return result;
}

static TmEcode JsonIKELogThreadInit(ThreadVars *t, const void *initdata, void **data)
{
    LogIKELogThread *thread = SCCalloc(1, sizeof(*thread));
    if (unlikely(thread == NULL)) {
        return TM_ECODE_FAILED;
    }

    if (initdata == NULL) {
        SCLogDebug("Error getting context for EveLogIKE.  \"initdata\" is NULL.");
        goto error_exit;
    }

    thread->buffer = MemBufferCreateNew(JSON_OUTPUT_BUFFER_SIZE);
    if (unlikely(thread->buffer == NULL)) {
        goto error_exit;
    }

    thread->ikelog_ctx = ((OutputCtx *)initdata)->data;
    thread->file_ctx = LogFileEnsureExists(thread->ikelog_ctx->file_ctx, t->id);
    if (!thread->file_ctx) {
        goto error_exit;
    }

    *data = (void *)thread;
    return TM_ECODE_OK;

error_exit:
    if (thread->buffer != NULL) {
        MemBufferFree(thread->buffer);
    }
    SCFree(thread);
    return TM_ECODE_FAILED;
}

static TmEcode JsonIKELogThreadDeinit(ThreadVars *t, void *data)
{
    LogIKELogThread *thread = (LogIKELogThread *)data;
    if (thread == NULL) {
        return TM_ECODE_OK;
    }
    if (thread->buffer != NULL) {
        MemBufferFree(thread->buffer);
    }
    SCFree(thread);
    return TM_ECODE_OK;
}

void JsonIKELogRegister(void)
{
    /* Register as an eve sub-module. */
    OutputRegisterTxSubModule(LOGGER_JSON_IKE, "eve-log", "JsonIKELog", "eve-log.ike",
            OutputIKELogInitSub, ALPROTO_IKE, JsonIKELogger, JsonIKELogThreadInit,
            JsonIKELogThreadDeinit, NULL);

    SCLogDebug("IKE JSON logger registered.");
}
