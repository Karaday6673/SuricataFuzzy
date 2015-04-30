/* Copyright (C) 2007-2014 Open Information Security Foundation
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
 * \author Tom DeCanio <td@npulsetech.com>
 *
 * Implements json common email logging portion of the engine.
 */

#include "suricata-common.h"
#include "debug.h"
#include "detect.h"
#include "pkt-var.h"
#include "conf.h"

#include "threads.h"
#include "threadvars.h"
#include "tm-threads.h"
#include "tm-threads-common.h"

#include "util-print.h"
#include "util-unittest.h"

#include "util-debug.h"
#include "app-layer-parser.h"
#include "output.h"
#include "app-layer-smtp.h"
#include "app-layer.h"
#include "util-privs.h"
#include "util-buffer.h"
#include "util-byte.h"

#include "util-logopenfile.h"
#include "util-crypt.h"

#include "output-json.h"
#include "output-json-email-common.h"

#ifdef HAVE_LIBJANSSON
#include <jansson.h>

/* JSON format logging */
json_t *JsonEmailLogJsonData(const Flow *f, void *state, void *vtx, uint64_t tx_id)
{
    SMTPState *smtp_state;
    MimeDecParseState *mime_state;
    MimeDecEntity *entity;

    json_t *sjs = json_object();
    if (sjs == NULL) {
        SCReturnPtr(NULL, "json_t");
    }

    /* check if we have SMTP state or not */
    AppProto proto = FlowGetAppProtocol(f);
    switch (proto) {
        case ALPROTO_SMTP:
            smtp_state = (SMTPState *)state;
            if (smtp_state == NULL) {
                SCLogDebug("no smtp state, so no request logging");
                SCReturnPtr(NULL, "json_t");
            }
            SMTPTransaction *tx = vtx;
            mime_state = tx->mime_state;
            entity = tx->msg_tail;
            SCLogDebug("lets go mime_state %p, entity %p, state_flag %u", mime_state, entity, mime_state ? mime_state->state_flag : 0);
            break;
        default:
            /* don't know how we got here */
            SCReturnPtr(NULL, "json_t");
    }
    if ((mime_state != NULL)) {
        if (entity == NULL) {
            SCReturnPtr(NULL, "json_t");
        }

#ifdef HAVE_NSS
        if (mime_state->md5_ctx && (mime_state->state_flag == PARSE_DONE)) {
            size_t x;
            int i;
            char s[256];
            if (likely(s != NULL)) {
                for (i = 0, x = 0; x < sizeof(mime_state->md5); x++) {
                    i += snprintf(s + i, 255-i, "%02x", mime_state->md5[x]);
                }
                json_object_set_new(sjs, "body_md5", json_string(s));
            }
        }
#endif

        json_object_set_new(sjs, "status",
                            json_string(MimeDecParseStateGetStatus(mime_state)));

        if ((entity->header_flags & HDR_IS_LOGGED) == 0) {
            MimeDecField *field;
            //printf("email LOG\n");

            /* From: */
            field = MimeDecFindField(entity, "from");
            if (field != NULL) {
                char *s = BytesToString((uint8_t *)field->value,
                                        (size_t)field->value_len);
                if (likely(s != NULL)) {
                    //printf("From: \"%s\"\n", s);
                    json_object_set_new(sjs, "from", json_string(s));
                    SCFree(s);
                }
            }

            /* To: */
            char *to_line = NULL;
            field = MimeDecFindField(entity, "to");
            if (field != NULL) {
                json_t *js_to = json_array();
                if (likely(js_to != NULL)) {
                    to_line = BytesToString((uint8_t *)field->value,
                                            (size_t)field->value_len);
                    if (likely(to_line != NULL)) {
                        char *savep = NULL;
                        char *p;
                        //printf("to_line:: TO: \"%s\" (%d)\n", to_line, strlen(to_line));
                        p = strtok_r(to_line, ",", &savep);
                        //printf("got another addr: \"%s\"\n", p);
                        json_array_append_new(js_to, json_string(p));
                        while ((p = strtok_r(NULL, ",", &savep)) != NULL) {
                            //printf("got another addr: \"%s\"\n", p);
                            json_array_append_new(js_to, json_string(&p[strspn(p, " ")]));
                        }
                        SCFree(to_line);
                    }
                    json_object_set_new(sjs, "to", js_to);
                }
            }

            /* Cc: */
            char *cc_line = NULL;
            field = MimeDecFindField(entity, "cc");
            if (field != NULL) {
                json_t *js_cc = json_array();
                if (likely(js_cc != NULL)) {
                    cc_line = BytesToString((uint8_t *)field->value,
                                            (size_t)field->value_len);
                    if (likely(cc_line != NULL)) {
                        char *savep = NULL;
                        char *p;
                        //printf("cc_line:: CC: \"%s\" (%d)\n", to_line, strlen(to_line));
                        p = strtok_r(cc_line, ",", &savep);
                        //printf("got another addr: \"%s\"\n", p);
                        json_array_append_new(js_cc, json_string(p));
                        while ((p = strtok_r(NULL, ",", &savep)) != NULL) {
                            //printf("got another addr: \"%s\"\n", p);
                            json_array_append_new(js_cc, json_string(&p[strspn(p, " ")]));
                        }
                        SCFree(cc_line);
                    }
                    json_object_set_new(sjs, "cc", js_cc);
                }
            }

            /* Subject: */
            field = MimeDecFindField(entity, "subject");
            if (field != NULL) {
                char *s = BytesToString((uint8_t *)field->value, (size_t) field->value_len);
                if (likely(s != NULL)) {
                    //printf("Subject: \"%s\"\n", s);
                    json_object_set_new(sjs, "subject", json_string(s));
                    SCFree(s);
                }
            }

            entity->header_flags |= HDR_IS_LOGGED;

            if (mime_state->stack == NULL || mime_state->stack->top == NULL || mime_state->stack->top->data == NULL)
                SCReturnPtr(NULL, "json_t");

            entity = (MimeDecEntity *)mime_state->stack->top->data;
            int attch_cnt = 0;
            int url_cnt = 0;
            json_t *js_attch = json_array();
            json_t *js_url = json_array();
            if (entity->url_list != NULL) {
                MimeDecUrl *url;
                for (url = entity->url_list; url != NULL; url = url->next) {
                    char *s = BytesToString((uint8_t *)url->url,
                                            (size_t)url->url_len);
                    if (s != NULL) {
                        //printf("URL: \"%s\"\n", s);
                        json_array_append_new(js_url,
                                          json_string(s));
                        SCFree(s);
                        url_cnt += 1;
                    }
                }
            }
            for (entity = entity->child; entity != NULL; entity = entity->next) {
                if (entity->ctnt_flags & CTNT_IS_ATTACHMENT) {

                    char *s = BytesToString((uint8_t *)entity->filename,
                                            (size_t)entity->filename_len);
                    //printf("found attachment \"%s\"\n", s);
                    json_array_append_new(js_attch,
                                          json_string(s));
                    SCFree(s);
                    attch_cnt += 1;
                }
                if (entity->url_list != NULL) {
                    MimeDecUrl *url;
                    for (url = entity->url_list; url != NULL; url = url->next) {
                        char *s = BytesToString((uint8_t *)url->url,
                                                (size_t)url->url_len);
                        if (s != NULL) {
                            //printf("URL: \"%s\"\n", s);
                            json_array_append_new(js_url,
                                              json_string(s));
                            SCFree(s);
                            url_cnt += 1;
                        }
                    }
                }
            }
            if (attch_cnt > 0) {
                json_object_set_new(sjs, "attachment", js_attch);
            } else {
                json_decref(js_attch);
            }
            if (url_cnt > 0) {
                json_object_set_new(sjs, "url", js_url);
            } else {
                json_decref(js_url);
            }
//            FLOWLOCK_UNLOCK(p->flow);
            SCReturnPtr(sjs, "json_t");
        }
    }

    json_decref(sjs);
//    FLOWLOCK_UNLOCK(p->flow);
    SCReturnPtr(NULL, "json_t");
}

/* JSON format logging */
TmEcode JsonEmailLogJson(JsonEmailLogThread *aft, json_t *js, const Packet *p, Flow *f, void *state, void *vtx, uint64_t tx_id)
{
    json_t *sjs = JsonEmailLogJsonData(f, state, vtx, tx_id);

    if (sjs) {
        json_object_set_new(js, "email", sjs);
        SCReturnInt(TM_ECODE_OK);
    } else
        SCReturnInt(TM_ECODE_FAILED);
}

json_t *JsonEmailAddMetadata(const Flow *f)
{
    SMTPState *smtp_state = (SMTPState *)FlowGetAppState(f);
    if (smtp_state) {
        uint64_t tx_id = AppLayerParserGetTransactionLogId(f->alparser);
        SMTPTransaction *tx = AppLayerParserGetTx(IPPROTO_TCP, ALPROTO_SMTP, smtp_state, tx_id);

        if (tx) {
            return JsonEmailLogJsonData(f, smtp_state, tx, tx_id);
        }
    }

    return NULL;
}


#endif
