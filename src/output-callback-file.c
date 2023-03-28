/**
 * \file
 *
 * \author Angelo Mirabella <mirabellaa@vmware.com>
 *
 * Generate fileinfo events and invoke corresponding callback.
 *
 */

#include "suricata-common.h"

#include "output-callback-file.h"

#include "output.h"
#include "output-callback.h"
#include "output-callback-http.h"
#include "threadvars.h"

#define MODULE_NAME       "CallbackFileLog"


/* Mock ThreadInit/DeInit methods.
 * Callbacks do not store any per-thread information. */
static TmEcode CallbackFileLogThreadInit(ThreadVars *t, const void *initdata, void **data) {
    return TM_ECODE_OK;
}

static TmEcode CallbackFileLogThreadDeinit(ThreadVars *t, void *data) {
    return TM_ECODE_OK;
}

static void FileGenerateEvent(const Packet *p, const File *ff, const uint64_t tx_id, uint32_t dir,
                              ThreadVars *tv) {
    /* TODO: add xff info? */
    FileinfoEvent event = {};
    enum OutputJsonLogDirection fdir = LOG_DIR_FLOW;

    switch (dir) {
        case STREAM_TOCLIENT:
            fdir = LOG_DIR_FLOW_TOCLIENT;
            break;
        case STREAM_TOSERVER:
            fdir = LOG_DIR_FLOW_TOSERVER;
            break;
        default:
            break;
    }

    EventAddCommonInfo(p, fdir, &event.common);

    /* TODO: add app layer metadata */
    switch (p->flow->alproto) {
        case ALPROTO_HTTP:
            ;
            HttpInfo *http = SCCalloc(1, sizeof(HttpInfo));
            if (http && CallbackHttpAddMetadata(p->flow, tx_id, http)) {
                event.app_layer.http = http;
            }
            break;
        default:
            break;
    }

    /* File info. */
    char name[ff->name_len + 1];

    memcpy(name, ff->name, ff->name_len);
    name[ff->name_len] = 0;
    event.fileinfo.filename = name;

#ifdef HAVE_MAGIC
    if (ff->magic)
        event.fileinfo.magic = ff->magic;
#endif

    event.fileinfo.gaps = ff->flags & FILE_HAS_GAPS;
    char md5[256];
    char sha1[256];
    switch (ff->state) {
        case FILE_STATE_CLOSED:
            event.fileinfo.state = "CLOSED";
            if (ff->flags & FILE_MD5) {
                size_t x;
                int i;
                for (i = 0, x = 0; x < sizeof(ff->md5); x++) {
                    i += snprintf(&md5[i], 255-i, "%02x", ff->md5[x]);
                }
                event.fileinfo.md5 = md5;
            }
            if (ff->flags & FILE_SHA1) {
                size_t x;
                int i;
                for (i = 0, x = 0; x < sizeof(ff->sha1); x++) {
                    i += snprintf(&sha1[i], 255-i, "%02x", ff->sha1[x]);
                }
                event.fileinfo.sha1 = sha1;
            }
            break;
        case FILE_STATE_TRUNCATED:
            event.fileinfo.state = "TRUNCATED";
            break;
        case FILE_STATE_ERROR:
            event.fileinfo.state = "ERROR";
            break;
        default:
            event.fileinfo.state = "UNKNOWN";
            break;
    }

    char sha256[256];
    if (ff->flags & FILE_SHA256) {
        size_t x;
        int i;
        for (i = 0, x = 0; x < sizeof(ff->sha256); x++) {
            i += snprintf(&sha256[i], 255-i, "%02x", ff->sha256[x]);
        }
        event.fileinfo.sha256 = sha256;
    }

    event.fileinfo.stored = ff->flags & FILE_STORED;
    if (event.fileinfo.stored) {
        event.fileinfo.file_id = ff->file_store_id;
    }

    event.fileinfo.size = FileTrackedSize(ff);
    if (ff->end > 0) {
        event.fileinfo.start = ff->start;
        event.fileinfo.end = ff->end;
    }

    /* TODO: add tx id? */

    /* Invoke callback and cleanup */
    tv->callbacks->fileinfo.func(&event, p->flow->tenant_uuid, tv->callbacks->fileinfo.user_ctx);
    if (event.app_layer.http) {
        CallbackHttpCleanupInfo(event.app_layer.http);
    }
}

static int CallbackFileLogger(ThreadVars *tv, void *thread_data, const Packet *p, const File *ff,
                              void *tx, const uint64_t tx_id, uint8_t dir) {
    BUG_ON(ff->flags & FILE_LOGGED);

    if (!tv->callbacks->fileinfo.func) {
        return 0;
    }

    /* TODO: add a filelog_ctx for flags such as stored only?
     * For now default behavior is to generate events only for stored files. */
    if ((ff->flags & FILE_STORED) == 0) {
        SCLogDebug("Not dumping information because file is not stored");
        return 0;
    }

    FileGenerateEvent(p, ff, tx_id, dir, tv);

    return 0;
}

void CallbackFileLogRegister(void) {
    OutputRegisterFileSubModule(LOGGER_CALLBACK_FILE, "", MODULE_NAME, "", NULL,
                                CallbackFileLogger, CallbackFileLogThreadInit,
                                CallbackFileLogThreadDeinit, NULL);
}
