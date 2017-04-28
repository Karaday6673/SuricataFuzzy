/* Copyright (C) 2017 Open Information Security Foundation
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

#ifndef __RUST_H__
#define __RUST_H__

typedef struct SuricataContext_ {
    SCError (*SCLogMessage)(const SCLogLevel, const char *, const unsigned int,
            const char *, const SCError, const char *message);
    void (*DetectEngineStateFree)(DetectEngineState *);
    void (*AppLayerDecoderEventsSetEventRaw)(AppLayerDecoderEvents **,
            uint8_t);
    void (*AppLayerDecoderEventsFreeEvents)(AppLayerDecoderEvents **);
} SuricataContext;

void rs_dns_init(void);

#endif /* !__RUST_H__ */
