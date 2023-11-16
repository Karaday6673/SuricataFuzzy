/* Copyright (C) 2007-2011 Open Information Security Foundation
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
 * \author Victor Julien <victor@inliniac.net>
 */

#ifndef __DETECT_ENGINE_FILE_H__
#define __DETECT_ENGINE_FILE_H__

uint8_t DetectFileInspectGeneric(DetectEngineCtx *de_ctx, DetectEngineThreadCtx *det_ctx,
        const struct DetectEngineAppInspectionEngine_ *engine, const Signature *s, Flow *f,
        uint8_t flags, void *_alstate, void *tx, uint64_t tx_id);

// file protocols with common file handling
typedef struct {
    AppProto al_proto;
    int direction;
    int to_client_progress;
    int to_server_progress;
} DetectFileHandlerProtocol_t;

void DetectFileRegisterProto(
        AppProto alproto, int direction, int to_client_progress, int to_server_progress);

#endif /* __DETECT_ENGINE_FILE_H__ */
