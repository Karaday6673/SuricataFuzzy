/* Copyright (C) 2015 Open Information Security Foundation
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
#include "stream.h"
#include "detect-engine-content-inspection.h"

#include "app-layer-dnp3.h"
#include "detect-dnp3-data.h"

int DetectEngineInspectDNP3Data(ThreadVars *tv, DetectEngineCtx *de_ctx,
    DetectEngineThreadCtx *det_ctx, Signature *s, Flow *f, uint8_t flags,
    void *alstate, void *txv, uint64_t tx_id)
{
    SCEnter();
    DNP3Transaction *tx = (DNP3Transaction *)txv;

    int r = 0;

    /* Content match - should probably be put into its own file. */
    if (flags & STREAM_TOSERVER && tx->request_buffer != NULL) {
        r = DetectEngineContentInspection(de_ctx, det_ctx, s,
            s->sm_lists[DETECT_SM_LIST_DNP3_DATA_MATCH], f, tx->request_buffer,
            tx->request_buffer_len, 0,
            DETECT_ENGINE_CONTENT_INSPECTION_MODE_DNP3_DATA, NULL);
    }
    else if (flags & STREAM_TOCLIENT && tx->response_buffer != NULL) {
        r = DetectEngineContentInspection(de_ctx, det_ctx, s,
            s->sm_lists[DETECT_SM_LIST_DNP3_DATA_MATCH], f, tx->response_buffer,
            tx->response_buffer_len, 0,
            DETECT_ENGINE_CONTENT_INSPECTION_MODE_DNP3_DATA, NULL);
    }

    SCReturnInt(r);
}
