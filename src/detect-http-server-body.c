/* Copyright (C) 2007-2018 Open Information Security Foundation
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
 * \ingroup httplayer
 *
 * @{
 */


/**
 * \file
 *
 * \author Anoop Saldanha <anoopsaldanha@gmail.com>
 * \author Victor Julien <victor@inliniac.net>
 *
 * Implements support for the http_server_body keyword
 */

#include "suricata-common.h"
#include "threads.h"
#include "decode.h"

#include "detect.h"
#include "detect-parse.h"
#include "detect-engine.h"
#include "detect-engine-mpm.h"
#include "detect-engine-state.h"
#include "detect-content.h"
#include "detect-pcre.h"

#include "flow.h"
#include "flow-var.h"
#include "flow-util.h"

#include "util-debug.h"
#include "util-unittest.h"
#include "util-unittest-helper.h"
#include "util-spm.h"

#include "app-layer.h"
#include "app-layer-parser.h"

#include "app-layer-htp.h"
#include "detect-http-server-body.h"
#include "stream-tcp.h"

static int DetectHttpServerBodySetup(DetectEngineCtx *, Signature *, const char *);
#ifdef UNITTESTS
static void DetectHttpServerBodyRegisterTests(void);
#endif
static int g_file_data_buffer_id = 0;

/**
 * \brief Registers the keyword handlers for the "http_server_body" keyword.
 */
void DetectHttpServerBodyRegister(void)
{
    sigmatch_table[DETECT_AL_HTTP_SERVER_BODY].name = "http_server_body";
    sigmatch_table[DETECT_AL_HTTP_SERVER_BODY].desc = "content modifier to match only on the HTTP response-body";
    sigmatch_table[DETECT_AL_HTTP_SERVER_BODY].url = DOC_URL DOC_VERSION "/rules/http-keywords.html#http-server-body";
    sigmatch_table[DETECT_AL_HTTP_SERVER_BODY].Setup = DetectHttpServerBodySetup;
#ifdef UNITTESTS
    sigmatch_table[DETECT_AL_HTTP_SERVER_BODY].RegisterTests = DetectHttpServerBodyRegisterTests;
#endif
    sigmatch_table[DETECT_AL_HTTP_SERVER_BODY].flags |= SIGMATCH_NOOPT;

    g_file_data_buffer_id = DetectBufferTypeRegister("file_data");
}

/**
 * \brief The setup function for the http_server_body keyword for a signature.
 *
 * \param de_ctx Pointer to the detection engine context.
 * \param s      Pointer to signature for the current Signature being parsed
 *               from the rules.
 * \param m      Pointer to the head of the SigMatchs for the current rule
 *               being parsed.
 * \param arg    Pointer to the string holding the keyword value.
 *
 * \retval  0 On success
 * \retval -1 On failure
 */
int DetectHttpServerBodySetup(DetectEngineCtx *de_ctx, Signature *s, const char *arg)
{
    return DetectEngineContentModifierBufferSetup(de_ctx, s, arg,
                                                  DETECT_AL_HTTP_SERVER_BODY,
                                                  g_file_data_buffer_id,
                                                  ALPROTO_HTTP);
}

/************************************Unittests*********************************/

#ifdef UNITTESTS
#include "tests/detect-http-server-body.c"
#endif /* UNITTESTS */

/**
 * @}
 */
