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

#include "suricata-common.h"

#include "detect.h"
#include "detect-engine-alert.h"
#include "detect-engine-threshold.h"
#include "detect-engine-tag.h"

#include "decode.h"

#include "flow.h"
#include "flow-private.h"

#include "util-profiling.h"

/** tag signature we use for tag alerts */
static Signature g_tag_signature;
/** tag packet alert structure for tag alerts */
static PacketAlert g_tag_pa;

void PacketAlertTagInit(void)
{
    memset(&g_tag_signature, 0x00, sizeof(g_tag_signature));

    g_tag_signature.id = TAG_SIG_ID;
    g_tag_signature.gid = TAG_SIG_GEN;
    g_tag_signature.num = TAG_SIG_ID;
    g_tag_signature.rev = 1;
    g_tag_signature.prio = 2;

    memset(&g_tag_pa, 0x00, sizeof(g_tag_pa));

    g_tag_pa.action = ACTION_ALERT;
    g_tag_pa.s = &g_tag_signature;
}

PacketAlert *PacketAlertGetTag(void)
{
    return &g_tag_pa;
}

/**
 * \brief Handle a packet and check if needs a threshold logic
 *        Also apply rule action if necessary.
 *
 * \param de_ctx Detection Context
 * \param sig Signature pointer
 * \param p Packet structure
 *
 * \retval 1 alert is not suppressed
 * \retval 0 alert is suppressed
 */
static int PacketAlertHandle(DetectEngineCtx *de_ctx, DetectEngineThreadCtx *det_ctx,
        const Signature *s, Packet *p, PacketAlert *pa)
{
    SCEnter();
    int ret = 1;
    const DetectThresholdData *td = NULL;
    const SigMatchData *smd;

    if (!(PKT_IS_IPV4(p) || PKT_IS_IPV6(p))) {
        SCReturnInt(1);
    }

    /* handle suppressions first */
    if (s->sm_arrays[DETECT_SM_LIST_SUPPRESS] != NULL) {
        KEYWORD_PROFILING_SET_LIST(det_ctx, DETECT_SM_LIST_SUPPRESS);
        smd = NULL;
        do {
            td = SigGetThresholdTypeIter(s, &smd, DETECT_SM_LIST_SUPPRESS);
            if (td != NULL) {
                SCLogDebug("td %p", td);

                /* PacketAlertThreshold returns 2 if the alert is suppressed but
                 * we do need to apply rule actions to the packet. */
                KEYWORD_PROFILING_START;
                ret = PacketAlertThreshold(de_ctx, det_ctx, td, p, s, pa);
                if (ret == 0 || ret == 2) {
                    KEYWORD_PROFILING_END(det_ctx, DETECT_THRESHOLD, 0);
                    /* It doesn't match threshold, remove it */
                    SCReturnInt(ret);
                }
                KEYWORD_PROFILING_END(det_ctx, DETECT_THRESHOLD, 1);
            }
        } while (smd != NULL);
    }

    /* if we're still here, consider thresholding */
    if (s->sm_arrays[DETECT_SM_LIST_THRESHOLD] != NULL) {
        KEYWORD_PROFILING_SET_LIST(det_ctx, DETECT_SM_LIST_THRESHOLD);
        smd = NULL;
        do {
            td = SigGetThresholdTypeIter(s, &smd, DETECT_SM_LIST_THRESHOLD);
            if (td != NULL) {
                SCLogDebug("td %p", td);

                /* PacketAlertThreshold returns 2 if the alert is suppressed but
                 * we do need to apply rule actions to the packet. */
                KEYWORD_PROFILING_START;
                ret = PacketAlertThreshold(de_ctx, det_ctx, td, p, s, pa);
                if (ret == 0 || ret == 2) {
                    KEYWORD_PROFILING_END(det_ctx, DETECT_THRESHOLD ,0);
                    /* It doesn't match threshold, remove it */
                    SCReturnInt(ret);
                }
                KEYWORD_PROFILING_END(det_ctx, DETECT_THRESHOLD, 1);
            }
        } while (smd != NULL);
    }
    SCReturnInt(1);
}


/**
 * \brief Check if a certain sid alerted, this is used in the test functions
 *
 * \param p   Packet on which we want to check if the signature alerted or not
 * \param sid Signature id of the signature that thas to be checked for a match
 *
 * \retval match A value > 0 on a match; 0 on no match
 */
int PacketAlertCheck(Packet *p, uint32_t sid)
{
    uint16_t i = 0;
    int match = 0;

    for (i = 0; i < p->alerts.cnt; i++) {
        if (p->alerts.alerts[i].s == NULL)
            continue;

        if (p->alerts.alerts[i].s->id == sid)
            match++;
    }

    return match;
}

static inline void RuleActionToFlow(const uint8_t action, Flow *f)
{
    if (action & (ACTION_DROP | ACTION_REJECT_ANY | ACTION_PASS)) {
        if (f->flags & (FLOW_ACTION_DROP | FLOW_ACTION_PASS)) {
            /* drop or pass already set. First to set wins. */
            SCLogDebug("not setting %s flow already set to %s",
                    (action & ACTION_PASS) ? "pass" : "drop",
                    (f->flags & FLOW_ACTION_DROP) ? "drop" : "pass");
        } else {
            if (action & (ACTION_DROP | ACTION_REJECT_ANY)) {
                f->flags |= FLOW_ACTION_DROP;
                SCLogDebug("setting flow action drop");
            }
            if (action & ACTION_PASS) {
                f->flags |= FLOW_ACTION_PASS;
                SCLogDebug("setting flow action pass");
                FlowSetNoPacketInspectionFlag(f);
            }
        }
    }
}

/** \brief Apply action(s) and Set 'drop' sig info,
 *         if applicable */
static void PacketApplySignatureActions(Packet *p, const Signature *s, const uint8_t alert_flags)
{
    SCLogDebug("packet %" PRIu64 " sid %u action %02x alert_flags %02x", p->pcap_cnt, s->id,
            s->action, alert_flags);
    PacketUpdateAction(p, s->action);

    if (s->action & ACTION_DROP) {
        if (p->alerts.drop.action == 0) {
            p->alerts.drop.num = s->num;
            p->alerts.drop.action = s->action;
            p->alerts.drop.s = (Signature *)s;
        }
        if ((p->flow != NULL) && (alert_flags & PACKET_ALERT_FLAG_APPLY_ACTION_TO_FLOW)) {
            RuleActionToFlow(s->action, p->flow);
        }
    } else if (s->action & ACTION_PASS) {
        if ((p->flow != NULL) && (alert_flags & PACKET_ALERT_FLAG_APPLY_ACTION_TO_FLOW)) {
            RuleActionToFlow(s->action, p->flow);
        }
    }
}

/**
 * \brief Check the threshold of the sigs that match, set actions, break on pass action
 *        This function iterate the packet alerts array, removing those that didn't match
 *        the threshold, and those that match after a signature with the action "pass".
 *        The array is sorted by action priority/order
 * \param de_ctx detection engine context
 * \param det_ctx detection engine thread context
 * \param p pointer to the packet
 */
void PacketAlertFinalize(DetectEngineCtx *de_ctx, DetectEngineThreadCtx *det_ctx, Packet *p)
{
    SCEnter();
    int i = 0;
    uint16_t max_pos = det_ctx->alert_queue_size;
    bool has_pass_action = false;

    while (i < max_pos) {
        const Signature *s = det_ctx->alert_queue[i].s;
        uint8_t res = PacketAlertHandle(de_ctx, det_ctx, s, p, &det_ctx->alert_queue[i]);

        if (res > 0) {
            /* Now, if we have an alert, we have to check if we want
             * to tag this session or src/dst host */
            if (s->sm_arrays[DETECT_SM_LIST_TMATCH] != NULL) {
                KEYWORD_PROFILING_SET_LIST(det_ctx, DETECT_SM_LIST_TMATCH);
                SigMatchData *smd = s->sm_arrays[DETECT_SM_LIST_TMATCH];
                while (1) {
                    /* tags are set only for alerts */
                    KEYWORD_PROFILING_START;
                    sigmatch_table[smd->type].Match(det_ctx, p, (Signature *)s, smd->ctx);
                    KEYWORD_PROFILING_END(det_ctx, smd->type, 1);
                    if (smd->is_last)
                        break;
                    smd++;
                }
            }

            /* For DROP and PASS sigs we need to apply the action to the flow if
             * - sig is IP or PD only
             * - match is in applayer
             * - match is in stream */
            if (s->action & (ACTION_DROP | ACTION_PASS)) {
                if ((det_ctx->alert_queue[i].flags &
                            (PACKET_ALERT_FLAG_STATE_MATCH | PACKET_ALERT_FLAG_STREAM_MATCH)) ||
                        (s->flags & (SIG_FLAG_IPONLY | SIG_FLAG_PDONLY | SIG_FLAG_APPLAYER))) {
                    det_ctx->alert_queue[i].flags |= PACKET_ALERT_FLAG_APPLY_ACTION_TO_FLOW;
                    SCLogDebug("packet %" PRIu64 " sid %u action %02x alert_flags %02x (set "
                               "PACKET_ALERT_FLAG_APPLY_ACTION_TO_FLOW)",
                            p->pcap_cnt, s->id, s->action, det_ctx->alert_queue[i].flags);
                }
            }

            /* set actions on packet */
            PacketApplySignatureActions(
                    p, det_ctx->alert_queue[i].s, det_ctx->alert_queue[i].flags);

            if (PacketTestAction(p, ACTION_PASS)) {
                /* Ok, reset the alert cnt to end in the previous of pass
                 * so we ignore the rest with less prio */
                has_pass_action = true;
            }
        }

        /* Thresholding removes this alert */
        if (res == 0 || res == 2 || (s->flags & SIG_FLAG_NOALERT)) {
            /* we will not copy this to the AlertQueue */
        } else {
            memcpy(&p->alerts.alerts[p->alerts.cnt], &det_ctx->alert_queue[i], sizeof(PacketAlert));
            SCLogDebug("Appending sid %" PRIu32 " alert to Packet::alerts at pos %u", s->id, i);
            if (has_pass_action) {
                break;
            }
            p->alerts.cnt++;
        }
        i++;
    }

    /* At this point, we should have all the new alerts. Now check the tag
     * keyword context for sessions and hosts */
    if (!(p->flags & PKT_PSEUDO_STREAM_END))
        TagHandlePacket(de_ctx, det_ctx, p);

    /* Set flag on flow to indicate that it has alerts */
    if (p->flow != NULL && p->alerts.cnt > 0) {
        FlowSetHasAlertsFlag(p->flow);
    }

}


