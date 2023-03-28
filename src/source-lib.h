/** \file
 *
 *  \author Angelo Mirabella <mirabellaa@vmware.com>
 */

#ifndef __SOURCE_LIB_H__
#define __SOURCE_LIB_H__

#include "threadvars.h"


/** \brief register a "Decode" module for suricata as a library.
 *
 *  The "Decode" module is the first module invoked when processing a packet */
void TmModuleDecodeLibRegister(void);

/** \brief process a single packet
 *
 * \param tv                    Pointer to the per-thread structure.
 * \param data                  Pointer to the raw packet.
 * \param datalink              Datalink type.
 * \param ts                    Timeval structure.
 * \param len                   Packet length.
 * \param ignore_pkt_checksum   Boolean indicating if we should ignore the packet checksum.
 * \param tenant_uuid           Tenant uuid (16 bytes) to associate a flow to a tenant.
 * \param tenant_id             Tenant id of hte detection engine to use.
 * \return                      Struct containing generated alerts if any.
 */
int TmModuleLibHandlePacket(ThreadVars *tv, const uint8_t *data, int datalink,
                            struct timeval ts, uint32_t len, int ignore_pkt_checksum,
                            uint64_t *tenant_uuid, uint32_t tenant_id);

#endif /* __SOURCE_LIB_H__ */
