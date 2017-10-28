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

/**
 * \file
 *
 * \author Pierre Chifflier <chifflier@wzdftpd.net>
 */

#ifndef __APP_LAYER_REGISTER_H__
#define __APP_LAYER_REGISTER_H__

typedef struct AppLayerParser {
    const char *name;
    const char *default_port;
    int ip_proto;
    const char *proto_name;

    ProbingParserFPtr ProbeTS;
    ProbingParserFPtr ProbeTC;

    uint16_t min_depth;
    uint16_t max_depth;

    void *(*StateAlloc)(void);
    void (*StateFree)(void *);

    AppLayerParserFPtr ParseTS;
    AppLayerParserFPtr ParseTC;

    uint64_t (*StateGetTxCnt)(void *alstate);
    void *(*StateGetTx)(void *alstate, uint64_t tx_id);
    void (*StateTransactionFree)(void *, uint64_t);
    int (*StateGetProgressCompletionStatus)(uint8_t direction);
    int (*StateGetProgress)(void *alstate, uint8_t direction);
    int (*StateGetTxLogged)(void *alstate, void *tx, uint32_t logger);
    void (*StateSetTxLogged)(void *alstate, void *tx, uint32_t logger);

    DetectEngineState *(*GetTxDetectState)(void *tx);
    int (*SetTxDetectState)(void *alstate, void *tx, DetectEngineState *);
    int (*StateHasTxDetectState)(void *alstate);

    int (*StateHasEvents)(void *);
    AppLayerDecoderEvents *(*StateGetEvents)(void *, uint64_t);
    int (*StateGetEventInfo)(const char *event_name,
                             int *event_id, AppLayerEventType *event_type);

    void *(*LocalStorageAlloc)(void);
    void (*LocalStorageFree)(void *);

    uint64_t (*GetTxMpmIDs)(void *tx);
    int (*SetTxMpmIDs)(void *tx, uint64_t);

    FileContainer *(*StateGetFiles)(void *, uint8_t);
} AppLayerParser;

AppProto AppLayerRegisterParser(const struct AppLayerParser *parser);

#endif /* __APP_LAYER_REGISTER_H__ */
