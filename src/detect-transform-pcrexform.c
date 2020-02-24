/* Copyright (C) 2020 Open Information Security Foundation
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
 * \author Jeff Lucovsky <jeff@lucovsky.org>
 *
 * Implements the pcrexform transform keyword with option support
 */

#include "suricata-common.h"

#include "detect.h"
#include "detect-engine.h"
#include "detect-parse.h"
#include "detect-transform-pcrexform.h"

typedef struct DetectTransformPcrexformData_ {
    pcre *parse_regex;
    pcre_extra *parse_regex_study;
} DetectTransformPcrexformData;

static int DetectTransformPcrexformSetup (DetectEngineCtx *, Signature *, const char *);
static void DetectTransformPcrexformFree(void *);
static void DetectTransformPcrexform(InspectionBuffer *buffer, void *options);

void DetectTransformPcrexformRegister(void)
{
    sigmatch_table[DETECT_TRANSFORM_PCREXFORM].name = "pcrexform";
    sigmatch_table[DETECT_TRANSFORM_PCREXFORM].desc =
        "modify buffer via PCRE before inspection";
    sigmatch_table[DETECT_TRANSFORM_PCREXFORM].url =
        DOC_URL DOC_VERSION "/rules/transforms.html#pcre-xform";
    sigmatch_table[DETECT_TRANSFORM_PCREXFORM].Transform =
        DetectTransformPcrexform;
    sigmatch_table[DETECT_TRANSFORM_PCREXFORM].Free =
        DetectTransformPcrexformFree;
    sigmatch_table[DETECT_TRANSFORM_PCREXFORM].Setup =
        DetectTransformPcrexformSetup;

    sigmatch_table[DETECT_TRANSFORM_PCREXFORM].flags |= SIGMATCH_QUOTES_OPTIONAL;
}

static void DetectTransformPcrexformFree(void *ptr)
{
    if (ptr != NULL) {
        DetectTransformPcrexformData *pxd = (DetectTransformPcrexformData *) ptr;
        SCFree(pxd);
    }
}
/**
 *  \internal
 *  \brief Apply the pcrexform keyword to the last pattern match
 *  \param det_ctx detection engine ctx
 *  \param s signature
 *  \param regexstr options string
 *  \retval 0 ok
 *  \retval -1 failure
 */
static int DetectTransformPcrexformSetup (DetectEngineCtx *de_ctx, Signature *s, const char *regexstr)
{
    SCEnter();

    // Create pxd from regexstr
    DetectTransformPcrexformData *pxd = SCCalloc(sizeof(*pxd), 1);
    if (pxd == NULL) {
        SCLogDebug("pxd allocation failed");
        SCReturnInt(-1);
    }

    DetectSetupParseRegexes(regexstr, &pxd->parse_regex, &pxd->parse_regex_study);

    int r = DetectSignatureAddTransform(s, DETECT_TRANSFORM_PCREXFORM, pxd);

    SCReturnInt(r);
}

static void DetectTransformPcrexform(InspectionBuffer *buffer, void *options)
{
#define MAX_SUBSTRINGS 100
    const char *input = (const char *)buffer->inspect;
    const uint32_t input_len = buffer->inspect_len;
    DetectTransformPcrexformData *pxd = options;

    int ov[MAX_SUBSTRINGS];
    int ret = pcre_exec(pxd->parse_regex, pxd->parse_regex_study, (char *)input,
                        input_len, 0, 0, ov, MAX_SUBSTRINGS);

    if (ret > 0) {
        char str[128];
        ret = pcre_copy_substring((char *) buffer->inspect, ov,
                                  MAX_SUBSTRINGS, ret - 1, str, sizeof(str));

        if (ret) {
            InspectionBufferCopy(buffer, (uint8_t *)str, (uint32_t) strlen(str));
        }
    }
}
