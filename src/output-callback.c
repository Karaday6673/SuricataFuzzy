/**
 * \file
 *
 * \author Angelo Mirabella <mirabellaa@vmware.com>
 *
 * Common utilities for event callbacks.
 *
 */

#include "suricata-common.h"
#include "app-layer-ftp.h"
#include "output-callback.h"
#include "output-callback-http.h"
#include "output-json-smb.h"
#include "output-json-smtp.h"
#include "app-layer-protos.h"
#include "rust.h"

/* Add information common to all events. */
void EventAddCommonInfo(const Packet *p, enum OutputJsonLogDirection dir, Common *common,
                        JsonAddrInfo *addr) {
    /* First initialize the address info (5-tuple). */
    JsonAddrInfoInit(p, LOG_DIR_PACKET, addr);
    common->src_ip = addr->src_ip;
    common->dst_ip = addr->dst_ip;
    common->sp = addr->sp;
    common->dp = addr->dp;
    common->proto = addr->proto;
    common->proto = addr->proto;

    /* Timestamp. */
    CreateIsoTimeString(p->ts, common->timestamp, sizeof(common->timestamp));

    /* Direction. */
    const char *direction = NULL;
    switch (dir) {
        case LOG_DIR_PACKET:
            if ((PKT_IS_TOCLIENT(p))) {
                direction = OUTPUT_DIR_PACKET_FLOW_TOCLIENT;
            } else {
                direction = OUTPUT_DIR_PACKET_FLOW_TOSERVER;
            }
            break;
        case LOG_DIR_FLOW:
        case LOG_DIR_FLOW_TOSERVER:
            direction = OUTPUT_DIR_PACKET_FLOW_TOSERVER;
            break;
        case LOG_DIR_FLOW_TOCLIENT:
            direction = OUTPUT_DIR_PACKET_FLOW_TOCLIENT;
            break;
        default:
            direction = "";
            break;
    }
    common->direction = direction;

    /* App layer protocol, if any. */
    if (p->flow != NULL) {
        const AppProto app_proto = FlowGetAppProtocol(p->flow);
        common->app_proto = app_proto ? AppProtoToString(app_proto) : "";
    }
}

/* Add app layer information (alert and fileinfo). */
void CallbackAddAppLayer(const Packet *p, const uint64_t tx_id, app_layer *app_layer) {
    const AppProto proto = FlowGetAppProtocol(p->flow);
    JsonBuilder *jb;

    switch (proto) {
        case ALPROTO_HTTP:
            ;
            const char *dir = NULL;
            if (PKT_IS_TOCLIENT(p)) {
                dir = LOG_HTTP_DIR_DOWNLOAD;
            } else {
                dir = LOG_HTTP_DIR_UPLOAD;
            }
            HttpInfo *http = SCCalloc(1, sizeof(HttpInfo));
            if (http && CallbackHttpAddMetadata(p->flow, tx_id, dir, http)) {
                app_layer->http = http;
            }
            break;
        case ALPROTO_SMB:
            ;
            jb = jb_new_object();
            if (EveSMBAddMetadata(p->flow, tx_id, jb)) {
                jb_close(jb);
                app_layer->nta = jb;
            } else {
                jb_free(jb);
            }
            break;
        case ALPROTO_FTPDATA:
            ;
            jb = jb_new_object();
            EveFTPDataAddMetadataDo(p->flow, jb);
            jb_close(jb);
            app_layer->nta = jb;
            break;
        case ALPROTO_SMTP:
            ;
            jb = jb_new_object();
            if (EveSMTPAddMetadata(p->flow, tx_id, jb)) {
                jb_close(jb);
                app_layer->nta = jb;
            } else {
                jb_free(jb);
            }
            /* TODO: Add email? */

        default:
            break;
    }
}

/* Free any memory allocated for app layer information (alert and fileinfo). */
void CallbackCleanupAppLayer(const Packet *p, const uint64_t tx_id, union app_layer *app_layer) {
    const AppProto proto = FlowGetAppProtocol(p->flow);
    switch (proto) {
        case ALPROTO_HTTP:
            if (app_layer->http) {
                SCFree(app_layer->http);
            }
            break;
        case ALPROTO_SMB:
        case ALPROTO_FTPDATA:
        case ALPROTO_SMTP:
            if (app_layer->nta) {
                SCFree(app_layer->nta);
            }
            break;
        default:
            break;
    }
}