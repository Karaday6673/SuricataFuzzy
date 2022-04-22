/* Copyright (C) 2021 Open Information Security Foundation
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
 * \author Lukas Sismis <lukas.sismis@gmail.com>
 */

#ifndef __SOURCE_DPDK_H__
#define __SOURCE_DPDK_H__

#include "queue.h"
#include "util-dpdk.h"

typedef enum { DPDK_COPY_MODE_NONE, DPDK_COPY_MODE_TAP, DPDK_COPY_MODE_IPS } DpdkCopyModeEnum;

#define DPDK_BURST_TX_WAIT_US 1

typedef enum {
    DPDK_ETHDEV_MODE, // run as DPDK primary process
    DPDK_RING_MODE,   // run as DPDK secondary process
} DpdkOperationMode;

/* DPDK Flags */
// General flags
#define DPDK_PROMISC   (1 << 0) /**< Promiscuous mode */
#define DPDK_MULTICAST (1 << 1) /**< Enable multicast packets */
// Offloads
#define DPDK_RX_CHECKSUM_OFFLOAD (1 << 4) /**< Enable chsum offload */

typedef struct DPDKIfaceConfig_ {
#ifdef HAVE_DPDK
    char iface[RTE_ETH_NAME_MAX_LEN];
    uint16_t port_id;
    uint16_t socket_id;
    DpdkOperationMode op_mode;
    /* number of threads - zero means all available */
    int threads;
    /* Ring mode settings */
    // Holds reference to all rx/tx rings, later assigned to workers
    struct rte_ring **rx_rings;
    struct rte_ring **tx_rings;
    /* End of ring mode settings */
    /* IPS mode */
    DpdkCopyModeEnum copy_mode;
    const char *out_iface;
    uint16_t out_port_id;
    /* DPDK flags */
    uint32_t flags;
    ChecksumValidationMode checksum_mode;
    /* set maximum transmission unit of the device in bytes */
    uint16_t mtu;
    uint16_t nb_rx_queues;
    uint16_t nb_rx_desc;
    uint16_t nb_tx_queues;
    uint16_t nb_tx_desc;
    uint32_t mempool_size;
    uint32_t mempool_cache_size;
    struct rte_mempool *pkt_mempool;
    SC_ATOMIC_DECLARE(unsigned int, ref);
    /* threads bind queue id one by one */
    SC_ATOMIC_DECLARE(uint16_t, queue_id);
    void (*DerefFunc)(void *);

    struct rte_flow *flow[100];
#endif
} DPDKIfaceConfig;

/**
 * \brief per packet DPDK vars
 *
 * This structure is used by the release data system and for IPS
 */
typedef struct DPDKPacketVars_ {
    struct rte_mbuf *mbuf;
    uint16_t out_port_id;
    uint16_t out_queue_id;
    uint8_t copy_mode;
    struct rte_ring *tx_ring; // pkt is sent to this ring (same as out_port_*)
} DPDKPacketVars;

void TmModuleReceiveDPDKRegister(void);
void TmModuleDecodeDPDKRegister(void);

#endif /* __SOURCE_DPDK_H__ */
