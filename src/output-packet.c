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
 * \author Victor Julien <victor@inliniac.net>
 *
 * Packet Logger Output registration functions
 */

#include "suricata-common.h"
#include "tm-modules.h"
#include "output-packet.h"

typedef struct OutputLoggerThreadStore_ {
    void *thread_data;
    struct OutputLoggerThreadStore_ *next;
} OutputLoggerThreadStore;

/** per thread data for this module, contains a list of per thread
 *  data for the packet loggers. */
typedef struct OutputLoggerThreadData_ {
    OutputLoggerThreadStore *store;
} OutputLoggerThreadData;

/* logger instance, a module + a output ctx,
 * it's perfectly valid that have multiple instances of the same
 * log module (e.g. fast.log) with different output ctx'. */
typedef struct OutputPacketLogger_ {
    PacketLogger LogFunc;
    PacketLogCondition ConditionFunc;
    OutputCtx *output_ctx;
    struct OutputPacketLogger_ *next;
    const char *name;
} OutputPacketLogger;

static OutputPacketLogger *list = NULL;

int OutputRegisterPacketLogger(char *name, PacketLogger LogFunc, PacketLogCondition ConditionFunc, OutputCtx *output_ctx) {
    OutputPacketLogger *o = SCMalloc(sizeof(*o));
    if (o == NULL)
        return -1;
    memset(o, 0x00, sizeof(*o));

    o->LogFunc = LogFunc;
    o->ConditionFunc = ConditionFunc;
    o->output_ctx = output_ctx;
    o->name = SCStrdup(name);
/* todo */

    if (list == NULL)
        list = o;
    else {
        OutputPacketLogger *t = list;
        while (t->next)
            t = t->next;
        t->next = o;
    }

    SCLogInfo("OutputRegisterPacketLogger happy");
    return 0;
}

static TmEcode OutputPacketLog(ThreadVars *tv, Packet *p, void *thread_data, PacketQueue *pq, PacketQueue *postpq) {
    BUG_ON(thread_data == NULL);
    BUG_ON(list == NULL);

    OutputLoggerThreadData *op_thread_data = (OutputLoggerThreadData *)thread_data;
    OutputPacketLogger *logger = list;
    OutputLoggerThreadStore *store = op_thread_data->store;

    BUG_ON(logger == NULL && store != NULL);
    BUG_ON(logger != NULL && store == NULL);
    BUG_ON(logger == NULL && store == NULL);

    while (logger && store) {
        BUG_ON(logger->LogFunc == NULL || logger->ConditionFunc == NULL);

        if ((logger->ConditionFunc(tv, (const Packet *)p)) == TRUE) {
            logger->LogFunc(tv, store->thread_data, (const Packet *)p);
        }

        logger = logger->next;
        store = store->next;

        BUG_ON(logger == NULL && store != NULL);
        BUG_ON(logger != NULL && store == NULL);
    }

    return TM_ECODE_OK;
}

/** \brief thread init for the packet logger
 *  This will run the thread init functions for the individual registered
 *  loggers */
static TmEcode OutputPacketLogThreadInit(ThreadVars *tv, void *initdata, void **data) {
    OutputLoggerThreadData *td = SCMalloc(sizeof(*td));
    if (td == NULL)
        return TM_ECODE_FAILED;
    memset(td, 0x00, sizeof(*td));

    *data = (void *)td;

    SCLogInfo("OutputPacketLogThreadInit happy (*data %p)", *data);

    OutputPacketLogger *logger = list;
    while (logger) {
        TmModule *tm_module = TmModuleGetByName((char *)logger->name);
        if (tm_module == NULL) {
            SCLogError(SC_ERR_INVALID_ARGUMENT,
                    "TmModuleGetByName for %s failed", logger->name);
            exit(EXIT_FAILURE);
        }

        if (tm_module->ThreadInit) {
            void *retptr = NULL;
            if (tm_module->ThreadInit(tv, (void *)logger->output_ctx, &retptr) == TM_ECODE_OK) {
                OutputLoggerThreadStore *ts = SCMalloc(sizeof(*ts));
/* todo */      BUG_ON(ts == NULL);
                memset(ts, 0x00, sizeof(*ts));

                /* store thread handle */
                ts->thread_data = retptr;

                if (td->store == NULL) {
                    td->store = ts;
                } else {
                    OutputLoggerThreadStore *tmp = td->store;
                    while (tmp->next != NULL)
                        tmp = tmp->next;
                    tmp->next = ts;
                }

                SCLogInfo("%s is now set up", logger->name);
            }
        }

        logger = logger->next;
    }

    return TM_ECODE_OK;
}

static TmEcode OutputPacketLogThreadDeinit(ThreadVars *tv, void *thread_data) {
    OutputLoggerThreadData *op_thread_data = (OutputLoggerThreadData *)thread_data;
    OutputLoggerThreadStore *store = op_thread_data->store;
    OutputPacketLogger *logger = list;

    while (logger && store) {
        TmModule *tm_module = TmModuleGetByName((char *)logger->name);
        if (tm_module == NULL) {
            SCLogError(SC_ERR_INVALID_ARGUMENT,
                    "TmModuleGetByName for %s failed", logger->name);
            exit(EXIT_FAILURE);
        }

        if (tm_module->ThreadDeinit) {
            tm_module->ThreadDeinit(tv, store->thread_data);
        }

        logger = logger->next;
        store = store->next;
    }
    return TM_ECODE_OK;
}

static void OutputPacketLogExitPrintStats(ThreadVars *tv, void *thread_data) {
    OutputLoggerThreadData *op_thread_data = (OutputLoggerThreadData *)thread_data;
    OutputLoggerThreadStore *store = op_thread_data->store;
    OutputPacketLogger *logger = list;

    while (logger && store) {
        TmModule *tm_module = TmModuleGetByName((char *)logger->name);
        if (tm_module == NULL) {
            SCLogError(SC_ERR_INVALID_ARGUMENT,
                    "TmModuleGetByName for %s failed", logger->name);
            exit(EXIT_FAILURE);
        }

        if (tm_module->ThreadExitPrintStats) {
            tm_module->ThreadExitPrintStats(tv, store->thread_data);
        }

        logger = logger->next;
        store = store->next;
    }
}

void TmModulePacketLoggerRegister (void) {
    tmm_modules[TMM_PACKETLOGGER].name = "__packet_logger__";
    tmm_modules[TMM_PACKETLOGGER].ThreadInit = OutputPacketLogThreadInit;
    tmm_modules[TMM_PACKETLOGGER].Func = OutputPacketLog;
    tmm_modules[TMM_PACKETLOGGER].ThreadExitPrintStats = OutputPacketLogExitPrintStats;
    tmm_modules[TMM_PACKETLOGGER].ThreadDeinit = OutputPacketLogThreadDeinit;
    tmm_modules[TMM_PACKETLOGGER].cap_flags = 0;
}

