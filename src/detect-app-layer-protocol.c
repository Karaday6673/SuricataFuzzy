/* Copyright (C) 2007-2013 Open Information Security Foundation
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
 */

#include "suricata-common.h"
#include "detect-engine.h"
#include "detect-parse.h"
#include "detect-app-layer-protocol.h"
#include "app-layer.h"
#include "app-layer-parser.h"
#include "util-debug.h"
#include "util-unittest.h"
#include "util-unittest-helper.h"

static void DetectAppLayerProtocolRegisterTests(void);

static int DetectAppLayerProtocolPacketMatch(ThreadVars *tv,
        DetectEngineThreadCtx *det_ctx,
        Packet *p, Signature *s, const SigMatchCtx *ctx)
{
    SCEnter();

    int r = 0;
    const DetectAppLayerProtocolData *data = (const DetectAppLayerProtocolData *)ctx;

    if ((p->flags & (PKT_PROTO_DETECT_TS_DONE|PKT_PROTO_DETECT_TC_DONE)) == 0) {
        SCLogNotice("packet %u: flags not set", (uint)p->pcap_cnt);
        SCReturnInt(0);
    }

    const Flow *f = p->flow;
    if (f == NULL) {
        SCLogNotice("packet %u: no flow", (uint)p->pcap_cnt);
        SCReturnInt(0);
    }

    if ((f->alproto_ts != ALPROTO_UNKNOWN) && (p->flowflags & FLOW_PKT_TOSERVER)) {
        SCLogNotice("toserver packet %u: looking for %u/neg %u, got %u", (uint)p->pcap_cnt,
                data->alproto, data->negated, f->alproto_ts);
        r = (data->negated) ? (f->alproto_ts != data->alproto) :
            (f->alproto_ts == data->alproto);
    } else if ((f->alproto_tc != ALPROTO_UNKNOWN) && (p->flowflags & FLOW_PKT_TOCLIENT)) {
        SCLogNotice("toclient packet %u: looking for %u/neg %u, got %u", (uint)p->pcap_cnt,
                data->alproto, data->negated, f->alproto_tc);
        r = (data->negated) ? (f->alproto_tc != data->alproto) :
            (f->alproto_tc == data->alproto);
    }

    SCReturnInt(r);
}

static DetectAppLayerProtocolData *DetectAppLayerProtocolParse(const char *arg)
{
    DetectAppLayerProtocolData *data;
    AppProto alproto = ALPROTO_UNKNOWN;
    uint8_t negated = 0;

    if (arg == NULL) {
        SCLogError(SC_ERR_INVALID_SIGNATURE, "app-layer-protocol keyword "
                   "supplied with no arguments.  This keyword needs "
                   "an argument.");
        return NULL;
    }

    while (*arg != '\0' && isspace((unsigned char)*arg))
        arg++;

    if (arg[0] == '!') {
        negated = 1;
        arg++;
    }

    while (*arg != '\0' && isspace((unsigned char)*arg))
        arg++;

    if (strcmp(arg, "failed") == 0) {
        alproto = ALPROTO_FAILED;
    } else {
        alproto = AppLayerGetProtoByName((char *)arg);
        if (alproto == ALPROTO_UNKNOWN) {
            SCLogError(SC_ERR_INVALID_SIGNATURE, "app-layer-protocol "
                    "keyword supplied with unknown protocol \"%s\"", arg);
            return NULL;
        }
    }

    data = SCMalloc(sizeof(DetectAppLayerProtocolData));
    if (unlikely(data == NULL))
        return NULL;
    data->alproto = alproto;
    data->negated = negated;

    return data;
}

static int DetectAppLayerProtocolSetup(DetectEngineCtx *de_ctx,
        Signature *s, char *arg)
{
    DetectAppLayerProtocolData *data = NULL;
    SigMatch *sm = NULL;

    if (s->alproto != ALPROTO_UNKNOWN) {
        SCLogError(SC_ERR_CONFLICTING_RULE_KEYWORDS, "Either we already "
                   "have the rule match on an app layer protocol set through "
                   "other keywords that match on this protocol, or have "
                   "already seen a non-negated app-layer-protocol.");
        goto error;
    }

    data = DetectAppLayerProtocolParse(arg);
    if (data == NULL)
        goto error;

    if (!data->negated && data->alproto != ALPROTO_FAILED) {
        SigMatch *sm = s->sm_lists[DETECT_SM_LIST_MATCH];
        for ( ; sm != NULL; sm = sm->next) {
            if (sm->type == DETECT_AL_APP_LAYER_PROTOCOL) {
                SCLogError(SC_ERR_CONFLICTING_RULE_KEYWORDS, "can't mix "
                        "positive app-layer-protocol match with negated "
                        "match or match for 'failed'.");
                goto error;
            }
        }

        s->alproto = data->alproto;
    }

    sm = SigMatchAlloc();
    if (sm == NULL)
        goto error;

    sm->type = DETECT_AL_APP_LAYER_PROTOCOL;
    sm->ctx = (void *)data;

    SCLogNotice("DETECT_SM_LIST_MATCH");
    SigMatchAppendSMToList(s, sm, DETECT_SM_LIST_MATCH);

    return 0;

error:
    if (data != NULL)
        SCFree(data);
    return -1;
}

static void DetectAppLayerProtocolFree(void *ptr)
{
    SCFree(ptr);
    return;
}

void DetectAppLayerProtocolRegister(void)
{
    sigmatch_table[DETECT_AL_APP_LAYER_PROTOCOL].name = "app-layer-protocol";
    sigmatch_table[DETECT_AL_APP_LAYER_PROTOCOL].Match =
        DetectAppLayerProtocolPacketMatch;
    sigmatch_table[DETECT_AL_APP_LAYER_PROTOCOL].Setup =
        DetectAppLayerProtocolSetup;
    sigmatch_table[DETECT_AL_APP_LAYER_PROTOCOL].Free =
        DetectAppLayerProtocolFree;
    sigmatch_table[DETECT_AL_APP_LAYER_PROTOCOL].RegisterTests =
        DetectAppLayerProtocolRegisterTests;

    return;
}

/**********************************Unittests***********************************/

#ifdef UNITTESTS

static int DetectAppLayerProtocolTest01(void)
{
    DetectAppLayerProtocolData *data = DetectAppLayerProtocolParse("http");
    FAIL_IF_NULL(data);
    FAIL_IF(data->alproto != ALPROTO_HTTP);
    FAIL_IF(data->negated != 0);
    DetectAppLayerProtocolFree(data);
    PASS;
}

static int DetectAppLayerProtocolTest02(void)
{
    DetectAppLayerProtocolData *data = DetectAppLayerProtocolParse("!http");
    FAIL_IF_NULL(data);
    FAIL_IF(data->alproto != ALPROTO_HTTP);
    FAIL_IF(data->negated == 0);
    DetectAppLayerProtocolFree(data);
    PASS;
}

static int DetectAppLayerProtocolTest03(void)
{
    Signature *s = NULL;
    DetectAppLayerProtocolData *data = NULL;
    DetectEngineCtx *de_ctx = DetectEngineCtxInit();
    FAIL_IF_NULL(de_ctx);
    de_ctx->flags |= DE_QUIET;

    s = DetectEngineAppendSig(de_ctx, "alert tcp any any -> any any "
            "(app-layer-protocol:http; sid:1;)");
    FAIL_IF_NULL(s);

    FAIL_IF(s->alproto != ALPROTO_HTTP);

    FAIL_IF_NULL(s->sm_lists[DETECT_SM_LIST_AMATCH]);
    FAIL_IF_NULL(s->sm_lists[DETECT_SM_LIST_AMATCH]->ctx);

    data = (DetectAppLayerProtocolData *)s->sm_lists[DETECT_SM_LIST_AMATCH]->ctx;
    FAIL_IF(data->alproto != ALPROTO_HTTP);
    FAIL_IF(data->negated);
    DetectEngineCtxFree(de_ctx);
    PASS;
}

static int DetectAppLayerProtocolTest04(void)
{
    Signature *s = NULL;
    DetectAppLayerProtocolData *data = NULL;
    DetectEngineCtx *de_ctx = DetectEngineCtxInit();
    FAIL_IF_NULL(de_ctx);
    de_ctx->flags |= DE_QUIET;

    s = DetectEngineAppendSig(de_ctx, "alert tcp any any -> any any "
            "(app-layer-protocol:!http; sid:1;)");
    FAIL_IF_NULL(s);
    FAIL_IF(s->alproto != ALPROTO_UNKNOWN);
    FAIL_IF(s->flags & SIG_FLAG_APPLAYER);

    /* negated match means we use MATCH not AMATCH */
    FAIL_IF_NOT(s->sm_lists[DETECT_SM_LIST_AMATCH] == NULL);
    FAIL_IF_NULL(s->sm_lists[DETECT_SM_LIST_MATCH]);
    FAIL_IF_NULL(s->sm_lists[DETECT_SM_LIST_MATCH]->ctx);

    data = (DetectAppLayerProtocolData*)s->sm_lists[DETECT_SM_LIST_MATCH]->ctx;
    FAIL_IF_NULL(data);
    FAIL_IF(data->alproto != ALPROTO_HTTP);
    FAIL_IF(data->negated == 0);

    DetectEngineCtxFree(de_ctx);
    PASS;
}

static int DetectAppLayerProtocolTest05(void)
{
    Signature *s = NULL;
    DetectAppLayerProtocolData *data = NULL;
    DetectEngineCtx *de_ctx = DetectEngineCtxInit();
    FAIL_IF_NULL(de_ctx);
    de_ctx->flags |= DE_QUIET;

    s = DetectEngineAppendSig(de_ctx, "alert tcp any any -> any any "
            "(app-layer-protocol:!http; app-layer-protocol:!smtp; sid:1;)");
    FAIL_IF_NULL(s);
    FAIL_IF(s->alproto != ALPROTO_UNKNOWN);
    FAIL_IF(s->flags & SIG_FLAG_APPLAYER);

    /* negated match means we use MATCH not AMATCH */
    FAIL_IF_NOT(s->sm_lists[DETECT_SM_LIST_AMATCH] == NULL);
    FAIL_IF_NULL(s->sm_lists[DETECT_SM_LIST_MATCH]);
    FAIL_IF_NULL(s->sm_lists[DETECT_SM_LIST_MATCH]->ctx);

    data = (DetectAppLayerProtocolData*)s->sm_lists[DETECT_SM_LIST_MATCH]->ctx;
    FAIL_IF_NULL(data);
    FAIL_IF(data->alproto != ALPROTO_HTTP);
    FAIL_IF(data->negated == 0);

    data = (DetectAppLayerProtocolData*)s->sm_lists[DETECT_SM_LIST_MATCH]->next->ctx;
    FAIL_IF_NULL(data);
    FAIL_IF(data->alproto != ALPROTO_SMTP);
    FAIL_IF(data->negated == 0);

    DetectEngineCtxFree(de_ctx);
    PASS;
}

static int DetectAppLayerProtocolTest06(void)
{
    Signature *s = NULL;
    DetectEngineCtx *de_ctx = DetectEngineCtxInit();
    FAIL_IF_NULL(de_ctx);
    de_ctx->flags |= DE_QUIET;

    s = DetectEngineAppendSig(de_ctx, "alert http any any -> any any "
            "(app-layer-protocol:smtp; sid:1;)");
    FAIL_IF_NOT_NULL(s);
    DetectEngineCtxFree(de_ctx);
    PASS;
}

static int DetectAppLayerProtocolTest07(void)
{
    Signature *s = NULL;
    DetectEngineCtx *de_ctx = DetectEngineCtxInit();
    FAIL_IF_NULL(de_ctx);
    de_ctx->flags |= DE_QUIET;

    s = DetectEngineAppendSig(de_ctx, "alert http any any -> any any "
            "(app-layer-protocol:!smtp; sid:1;)");
    FAIL_IF_NOT_NULL(s);
    DetectEngineCtxFree(de_ctx);
    PASS;
}

static int DetectAppLayerProtocolTest08(void)
{
    Signature *s = NULL;
    DetectEngineCtx *de_ctx = DetectEngineCtxInit();
    FAIL_IF_NULL(de_ctx);
    de_ctx->flags |= DE_QUIET;

    s = DetectEngineAppendSig(de_ctx, "alert tcp any any -> any any "
            "(app-layer-protocol:!smtp; app-layer-protocol:http; sid:1;)");
    FAIL_IF_NOT_NULL(s);
    DetectEngineCtxFree(de_ctx);
    PASS;
}

static int DetectAppLayerProtocolTest09(void)
{
    Signature *s = NULL;
    DetectEngineCtx *de_ctx = DetectEngineCtxInit();
    FAIL_IF_NULL(de_ctx);
    de_ctx->flags |= DE_QUIET;

    s = DetectEngineAppendSig(de_ctx, "alert tcp any any -> any any "
            "(app-layer-protocol:http; app-layer-protocol:!smtp; sid:1;)");
    FAIL_IF_NOT_NULL(s);
    DetectEngineCtxFree(de_ctx);
    PASS;
}

static int DetectAppLayerProtocolTest10(void)
{
    Signature *s = NULL;
    DetectEngineCtx *de_ctx = DetectEngineCtxInit();
    FAIL_IF_NULL(de_ctx);
    de_ctx->flags |= DE_QUIET;

    s = DetectEngineAppendSig(de_ctx, "alert tcp any any -> any any "
            "(app-layer-protocol:smtp; app-layer-protocol:!http; sid:1;)");
    FAIL_IF_NOT_NULL(s);
    DetectEngineCtxFree(de_ctx);
    PASS;
}

static int DetectAppLayerProtocolTest11(void)
{
    DetectAppLayerProtocolData *data = DetectAppLayerProtocolParse("failed");
    FAIL_IF_NULL(data);
    FAIL_IF(data->alproto != ALPROTO_FAILED);
    FAIL_IF(data->negated != 0);
    DetectAppLayerProtocolFree(data);
    PASS;
}

static int DetectAppLayerProtocolTest12(void)
{
    DetectAppLayerProtocolData *data = DetectAppLayerProtocolParse("!failed");
    FAIL_IF_NULL(data);
    FAIL_IF(data->alproto != ALPROTO_FAILED);
    FAIL_IF(data->negated == 0);
    DetectAppLayerProtocolFree(data);
    PASS;
}

static int DetectAppLayerProtocolTest13(void)
{
    Signature *s = NULL;
    DetectAppLayerProtocolData *data = NULL;
    DetectEngineCtx *de_ctx = DetectEngineCtxInit();
    FAIL_IF_NULL(de_ctx);
    de_ctx->flags |= DE_QUIET;

    s = DetectEngineAppendSig(de_ctx, "alert tcp any any -> any any "
            "(app-layer-protocol:failed; sid:1;)");
    FAIL_IF_NULL(s);

    FAIL_IF(s->alproto != ALPROTO_UNKNOWN);

    FAIL_IF_NULL(s->sm_lists[DETECT_SM_LIST_MATCH]);
    FAIL_IF_NULL(s->sm_lists[DETECT_SM_LIST_MATCH]->ctx);

    data = (DetectAppLayerProtocolData *)s->sm_lists[DETECT_SM_LIST_MATCH]->ctx;
    FAIL_IF(data->alproto != ALPROTO_FAILED);
    FAIL_IF(data->negated);
    DetectEngineCtxFree(de_ctx);
    PASS;
}

#endif /* UNITTESTS */

static void DetectAppLayerProtocolRegisterTests(void)
{
#ifdef UNITTESTS /* UNITTESTS */
    UtRegisterTest("DetectAppLayerProtocolTest01",
                   DetectAppLayerProtocolTest01);
    UtRegisterTest("DetectAppLayerProtocolTest02",
                   DetectAppLayerProtocolTest02);
    UtRegisterTest("DetectAppLayerProtocolTest03",
                   DetectAppLayerProtocolTest03);
    UtRegisterTest("DetectAppLayerProtocolTest04",
                   DetectAppLayerProtocolTest04);
    UtRegisterTest("DetectAppLayerProtocolTest05",
                   DetectAppLayerProtocolTest05);
    UtRegisterTest("DetectAppLayerProtocolTest06",
                   DetectAppLayerProtocolTest06);
    UtRegisterTest("DetectAppLayerProtocolTest07",
                   DetectAppLayerProtocolTest07);
    UtRegisterTest("DetectAppLayerProtocolTest08",
                   DetectAppLayerProtocolTest08);
    UtRegisterTest("DetectAppLayerProtocolTest09",
                   DetectAppLayerProtocolTest09);
    UtRegisterTest("DetectAppLayerProtocolTest10",
                   DetectAppLayerProtocolTest10);
    UtRegisterTest("DetectAppLayerProtocolTest11",
                   DetectAppLayerProtocolTest11);
    UtRegisterTest("DetectAppLayerProtocolTest12",
                   DetectAppLayerProtocolTest12);
    UtRegisterTest("DetectAppLayerProtocolTest13",
                   DetectAppLayerProtocolTest13);
#endif /* UNITTESTS */

    return;
}
