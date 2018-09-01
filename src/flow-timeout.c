/* Copyright (C) 2007-2017 Open Information Security Foundation
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
#include "suricata.h"
#include "decode.h"
#include "conf.h"
#include "threadvars.h"
#include "tm-threads.h"
#include "runmodes.h"

#include "util-random.h"
#include "util-time.h"

#include "flow.h"
#include "flow-queue.h"
#include "flow-hash.h"
#include "flow-util.h"
#include "flow-var.h"
#include "flow-private.h"
#include "flow-manager.h"
#include "flow-timeout.h"
#include "pkt-var.h"
#include "host.h"

#include "stream-tcp-private.h"
#include "stream-tcp-reassemble.h"
#include "stream-tcp.h"

#include "util-unittest.h"
#include "util-unittest-helper.h"
#include "util-byte.h"

#include "util-debug.h"
#include "util-privs.h"

#include "detect.h"
#include "detect-engine-state.h"
#include "stream.h"

#include "app-layer-parser.h"
#include "app-layer.h"

#include "util-profiling.h"

/**
 * \internal
 * \brief Pseudo packet setup for flow forced reassembly.
 *
 * \param direction Direction of the packet.  0 indicates toserver and 1
 *                  indicates toclient.
 * \param f         Pointer to the flow.
 * \param ssn       Pointer to the tcp session.
 * \param dummy     Indicates to create a dummy pseudo packet.  Not all pseudo
 *                  packets need to force reassembly, in which case we just
 *                  set dummy ack/seq values.
 */
static inline Packet *FlowForceReassemblyPseudoPacketSetup(Packet *p,
                                                           int direction,
                                                           Flow *f,
                                                           TcpSession *ssn)
{
    p->tenant_id = f->tenant_id;
    p->datalink = DLT_RAW;
    p->proto = IPPROTO_TCP;
    FlowReference(&p->flow, f);
    p->flags |= PKT_STREAM_EST;
    p->flags |= PKT_STREAM_EOF;
    p->flags |= PKT_HAS_FLOW;
    p->flags |= PKT_PSEUDO_STREAM_END;

    if (f->vlan_id[0] > 0) {
        p->vlan_id[0] = f->vlan_id[0];
        p->vlan_idx=1;

        if (f->vlan_id[1] > 0) {
            p->vlan_id[1] = f->vlan_id[1];
            p->vlan_idx=2;
        }
    }

    p->livedev = f->livedev;

    if (f->flags & FLOW_NOPACKET_INSPECTION) {
        DecodeSetNoPacketInspectionFlag(p);
    }
    if (f->flags & FLOW_NOPAYLOAD_INSPECTION) {
        DecodeSetNoPayloadInspectionFlag(p);
    }

    if (direction == 0)
        p->flowflags |= FLOW_PKT_TOSERVER;
    else
        p->flowflags |= FLOW_PKT_TOCLIENT;
    p->flowflags |= FLOW_PKT_ESTABLISHED;
    p->payload = NULL;
    p->payload_len = 0;

    if (FLOW_IS_IPV4(f)) {
        if (direction == 0) {
            FLOW_COPY_IPV4_ADDR_TO_PACKET(&f->src, &p->src);
            FLOW_COPY_IPV4_ADDR_TO_PACKET(&f->dst, &p->dst);
            p->sp = f->sp;
            p->dp = f->dp;
        } else {
            FLOW_COPY_IPV4_ADDR_TO_PACKET(&f->src, &p->dst);
            FLOW_COPY_IPV4_ADDR_TO_PACKET(&f->dst, &p->src);
            p->sp = f->dp;
            p->dp = f->sp;
        }

        /* Check if we have enough room in direct data. We need ipv4 hdr + tcp hdr.
         * Force an allocation if it is not the case.
         */
        if (GET_PKT_DIRECT_MAX_SIZE(p) <  40) {
            if (PacketCallocExtPkt(p, 40) == -1) {
                goto error;
            }
        }
        /* set the ip header */
        p->ip4h = (IPV4Hdr *)GET_PKT_DATA(p);
        /* version 4 and length 20 bytes for the tcp header */
        p->ip4h->ip_verhl = 0x45;
        p->ip4h->ip_tos = 0;
        p->ip4h->ip_len = htons(40);
        p->ip4h->ip_id = 0;
        p->ip4h->ip_off = 0;
        p->ip4h->ip_ttl = 64;
        p->ip4h->ip_proto = IPPROTO_TCP;
        //p->ip4h->ip_csum =
        if (direction == 0) {
            p->ip4h->s_ip_src.s_addr = f->src.addr_data32[0];
            p->ip4h->s_ip_dst.s_addr = f->dst.addr_data32[0];
        } else {
            p->ip4h->s_ip_src.s_addr = f->dst.addr_data32[0];
            p->ip4h->s_ip_dst.s_addr = f->src.addr_data32[0];
        }

        /* set the tcp header */
        p->tcph = (TCPHdr *)((uint8_t *)GET_PKT_DATA(p) + 20);

        SET_PKT_LEN(p, 40); /* ipv4 hdr + tcp hdr */

    } else if (FLOW_IS_IPV6(f)) {
        if (direction == 0) {
            FLOW_COPY_IPV6_ADDR_TO_PACKET(&f->src, &p->src);
            FLOW_COPY_IPV6_ADDR_TO_PACKET(&f->dst, &p->dst);
            p->sp = f->sp;
            p->dp = f->dp;
        } else {
            FLOW_COPY_IPV6_ADDR_TO_PACKET(&f->src, &p->dst);
            FLOW_COPY_IPV6_ADDR_TO_PACKET(&f->dst, &p->src);
            p->sp = f->dp;
            p->dp = f->sp;
        }

        /* Check if we have enough room in direct data. We need ipv6 hdr + tcp hdr.
         * Force an allocation if it is not the case.
         */
        if (GET_PKT_DIRECT_MAX_SIZE(p) <  60) {
            if (PacketCallocExtPkt(p, 60) == -1) {
                goto error;
            }
        }
        /* set the ip header */
        p->ip6h = (IPV6Hdr *)GET_PKT_DATA(p);
        /* version 6 */
        p->ip6h->s_ip6_vfc = 0x60;
        p->ip6h->s_ip6_flow = 0;
        p->ip6h->s_ip6_nxt = IPPROTO_TCP;
        p->ip6h->s_ip6_plen = htons(20);
        p->ip6h->s_ip6_hlim = 64;
        if (direction == 0) {
            p->ip6h->s_ip6_src[0] = f->src.addr_data32[0];
            p->ip6h->s_ip6_src[1] = f->src.addr_data32[1];
            p->ip6h->s_ip6_src[2] = f->src.addr_data32[2];
            p->ip6h->s_ip6_src[3] = f->src.addr_data32[3];
            p->ip6h->s_ip6_dst[0] = f->dst.addr_data32[0];
            p->ip6h->s_ip6_dst[1] = f->dst.addr_data32[1];
            p->ip6h->s_ip6_dst[2] = f->dst.addr_data32[2];
            p->ip6h->s_ip6_dst[3] = f->dst.addr_data32[3];
        } else {
            p->ip6h->s_ip6_src[0] = f->dst.addr_data32[0];
            p->ip6h->s_ip6_src[1] = f->dst.addr_data32[1];
            p->ip6h->s_ip6_src[2] = f->dst.addr_data32[2];
            p->ip6h->s_ip6_src[3] = f->dst.addr_data32[3];
            p->ip6h->s_ip6_dst[0] = f->src.addr_data32[0];
            p->ip6h->s_ip6_dst[1] = f->src.addr_data32[1];
            p->ip6h->s_ip6_dst[2] = f->src.addr_data32[2];
            p->ip6h->s_ip6_dst[3] = f->src.addr_data32[3];
        }

        /* set the tcp header */
        p->tcph = (TCPHdr *)((uint8_t *)GET_PKT_DATA(p) + 40);

        SET_PKT_LEN(p, 60); /* ipv6 hdr + tcp hdr */
    }

    p->tcph->th_offx2 = 0x50;
    p->tcph->th_flags |= TH_ACK;
    p->tcph->th_win = 10;
    p->tcph->th_urp = 0;

    /* to server */
    if (direction == 0) {
        p->tcph->th_sport = htons(f->sp);
        p->tcph->th_dport = htons(f->dp);

        p->tcph->th_seq = htonl(ssn->client.next_seq);
        p->tcph->th_ack = htonl(ssn->server.last_ack);

        /* to client */
    } else {
        p->tcph->th_sport = htons(f->dp);
        p->tcph->th_dport = htons(f->sp);

        p->tcph->th_seq = htonl(ssn->server.next_seq);
        p->tcph->th_ack = htonl(ssn->client.last_ack);
    }

    if (FLOW_IS_IPV4(f)) {
        p->tcph->th_sum = TCPChecksum(p->ip4h->s_ip_addrs,
                                               (uint16_t *)p->tcph, 20, 0);
        /* calc ipv4 csum as we may log it and barnyard might reject
         * a wrong checksum */
        p->ip4h->ip_csum = IPV4Checksum((uint16_t *)p->ip4h,
                IPV4_GET_RAW_HLEN(p->ip4h), 0);
    } else if (FLOW_IS_IPV6(f)) {
        p->tcph->th_sum = TCPChecksum(p->ip6h->s_ip6_addrs,
                                              (uint16_t *)p->tcph, 20, 0);
    }

    memset(&p->ts, 0, sizeof(struct timeval));
    TimeGet(&p->ts);

    AppLayerParserSetEOF(f->alparser);

    return p;

error:
    FlowDeReference(&p->flow);
    return NULL;
}

static inline Packet *FlowForceReassemblyPseudoPacketGet(int direction,
                                                         Flow *f,
                                                         TcpSession *ssn)
{
    PacketPoolWait();
    Packet *p = PacketPoolGetPacket();
    if (p == NULL) {
        return NULL;
    }

    PACKET_PROFILING_START(p);

    return FlowForceReassemblyPseudoPacketSetup(p, direction, f, ssn);
}

/**
 *  \brief Check if a flow needs forced reassembly, or any other processing
 *
 *  \param f *LOCKED* flow
 *  \param server ptr to int that should be set to 1 or 2 if we return 1
 *  \param client ptr to int that should be set to 1 or 2 if we return 1
 *
 *  \retval 0 no
 *  \retval 1 yes
 */
int FlowForceReassemblyNeedReassembly(Flow *f, int *server, int *client)
{
    if (f == NULL || f->protoctx == NULL) {
        *server = *client = STREAM_HAS_UNPROCESSED_SEGMENTS_NONE;
        SCReturnInt(0);
    }

    TcpSession *ssn = (TcpSession *)f->protoctx;
    *client = StreamNeedsReassembly(ssn, STREAM_TOSERVER);
    *server = StreamNeedsReassembly(ssn, STREAM_TOCLIENT);

    /* if state is not fully closed we assume that we haven't fully
     * inspected the app layer state yet */
    if (ssn->state >= TCP_ESTABLISHED && ssn->state != TCP_CLOSED)
    {
        *client = STREAM_HAS_UNPROCESSED_SEGMENTS_NEED_ONLY_DETECTION;
        *server = STREAM_HAS_UNPROCESSED_SEGMENTS_NEED_ONLY_DETECTION;
    }

    /* if app layer still needs some love, push through */
    if (f->alproto != ALPROTO_UNKNOWN && f->alstate != NULL &&
        AppLayerParserProtocolSupportsTxs(f->proto, f->alproto))
    {
        const uint64_t total_txs = AppLayerParserGetTxCnt(f, f->alstate);

        if (AppLayerParserGetTransactionActive(f, f->alparser, STREAM_TOCLIENT) < total_txs)
        {
            *server = STREAM_HAS_UNPROCESSED_SEGMENTS_NEED_ONLY_DETECTION;
        }
        if (AppLayerParserGetTransactionActive(f, f->alparser, STREAM_TOSERVER) < total_txs)
        {
            *client = STREAM_HAS_UNPROCESSED_SEGMENTS_NEED_ONLY_DETECTION;
        }
    }

    /* nothing to do */
    if (*client == STREAM_HAS_UNPROCESSED_SEGMENTS_NONE &&
        *server == STREAM_HAS_UNPROCESSED_SEGMENTS_NONE) {
        SCReturnInt(0);
    }

    SCReturnInt(1);
}

/**
 * \internal
 * \brief Forces reassembly for flow if it needs it.
 *
 *        The function requires flow to be locked beforehand.
 *
 * \param f Pointer to the flow.
 * \param server action required for server: 1 or 2
 * \param client action required for client: 1 or 2
 *
 * \retval 0 This flow doesn't need any reassembly processing; 1 otherwise.
 */
int FlowForceReassemblyForFlow(Flow *f, int server, int client)
{
    Packet *p1 = NULL, *p2 = NULL;

    /* looks like we have no flows in this queue */
    if (f == NULL || f->protoctx == NULL) {
        return 0;
    }

    /* Get the tcp session for the flow */
    TcpSession *ssn = (TcpSession *)f->protoctx;

    /* The packets we use are based on what segments in what direction are
     * unprocessed.
     * p1 if we have client segments for reassembly purpose only.  If we
     * have no server segments p2 can be a toserver packet with dummy
     * seq/ack, and if we have server segments p2 has to carry out reassembly
     * for server segment as well, in which case we will also need a p3 in the
     * toclient which is now dummy since all we need it for is detection */

    /* insert a pseudo packet in the toserver direction */
    if (client == STREAM_HAS_UNPROCESSED_SEGMENTS_NEED_ONLY_DETECTION) {
        p1 = FlowForceReassemblyPseudoPacketGet(0, f, ssn);
        if (p1 == NULL) {
            goto done;
        }
        PKT_SET_SRC(p1, PKT_SRC_FFR);

        if (server == STREAM_HAS_UNPROCESSED_SEGMENTS_NEED_ONLY_DETECTION) {
            p2 = FlowForceReassemblyPseudoPacketGet(1, f, ssn);
            if (p2 == NULL) {
                FlowDeReference(&p1->flow);
                TmqhOutputPacketpool(NULL, p1);
                goto done;
            }
            PKT_SET_SRC(p2, PKT_SRC_FFR);
        }
    } else {
        if (server == STREAM_HAS_UNPROCESSED_SEGMENTS_NEED_ONLY_DETECTION) {
            p1 = FlowForceReassemblyPseudoPacketGet(1, f, ssn);
            if (p1 == NULL) {
                goto done;
            }
            PKT_SET_SRC(p1, PKT_SRC_FFR);
        } else {
            /* impossible */
            BUG_ON(1);
        }
    }

    /* inject the packet(s) into the appropriate thread */
    int thread_id = (int)f->thread_id;
    Packet *packets[3] = { p1, p2 ? p2 : NULL, NULL }; /**< null terminated array of packets */
    if (unlikely(!(TmThreadsInjectPacketsById(packets, thread_id)))) {
        FlowDeReference(&p1->flow);
        TmqhOutputPacketpool(NULL, p1);
        if (p2) {
            FlowDeReference(&p2->flow);
            TmqhOutputPacketpool(NULL, p2);
        }
    }

    /* done, in case of error (no packet) we still tag flow as complete
     * as we're probably resource stress if we couldn't get packets */
done:
    f->flags |= FLOW_TIMEOUT_REASSEMBLY_DONE;
    return 1;
}

/**
 * \internal
 * \brief Forces reassembly for flows that need it.
 *
 * When this function is called we're running in virtually dead engine,
 * so locking the flows is not strictly required. The reasons it is still
 * done are:
 * - code consistency
 * - silence complaining profilers
 * - allow us to aggressively check using debug valdation assertions
 * - be robust in case of future changes
 * - locking overhead if neglectable when no other thread fights us
 *
 * \param q The queue to process flows from.
 */
static inline void FlowForceReassemblyForHash(void)
{
    for (uint32_t idx = 0; idx < flow_config.hash_size; idx++) {
        FlowBucket *fb = &flow_hash[idx];

        PacketPoolWaitForN(9);
        FBLOCK_LOCK(fb);

        /* get the topmost flow from the QUEUE */
        Flow *f = fb->head;

        /* we need to loop through all the flows in the queue */
        while (f != NULL) {
            PacketPoolWaitForN(3);

            FLOWLOCK_WRLOCK(f);

            /* Get the tcp session for the flow */
            TcpSession *ssn = (TcpSession *)f->protoctx;
            /* \todo Also skip flows that shouldn't be inspected */
            if (ssn == NULL) {
                FLOWLOCK_UNLOCK(f);
                f = f->hnext;
                continue;
            }

            int client_ok = 0;
            int server_ok = 0;
            if (FlowForceReassemblyNeedReassembly(f, &server_ok, &client_ok) == 1) {
                FlowForceReassemblyForFlow(f, server_ok, client_ok);
            }

            FLOWLOCK_UNLOCK(f);

            /* next flow in the queue */
            f = f->hnext;
        }
        FBLOCK_UNLOCK(fb);
    }
    return;
}

/**
 * \brief Force reassembly for all the flows that have unprocessed segments.
 */
void FlowForceReassembly(void)
{
    /* Carry out flow reassembly for unattended flows */
    FlowForceReassemblyForHash();
    return;
}

