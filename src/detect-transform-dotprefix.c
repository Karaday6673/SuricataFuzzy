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
 * Implements the dotprefix transformation
 */

#include "suricata-common.h"

#include "detect.h"
#include "detect-engine.h"
#include "detect-engine-prefilter.h"
#include "detect-parse.h"
#include "detect-transform-dotprefix.h"

#include "util-unittest.h"
#include "util-print.h"
#include "util-memrchr.h"
#include "util-memcpy.h"

static int DetectTransformDotPrefixSetup (DetectEngineCtx *, Signature *, const char *);
static void DetectTransformDotPrefixRegisterTests(void);

static void TransformDotPrefix(InspectionBuffer *buffer, void *options);
static void DetectTransformDotPrefixFree(void *);

void DetectTransformDotPrefixRegister(void)
{
    sigmatch_table[DETECT_TRANSFORM_DOTPREFIX].name = "dotprefix";
    sigmatch_table[DETECT_TRANSFORM_DOTPREFIX].desc =
        "modify buffer to extract the dotprefix";
    sigmatch_table[DETECT_TRANSFORM_DOTPREFIX].url =
        DOC_URL DOC_VERSION "/rules/transforms.html#dotprefix";
    sigmatch_table[DETECT_TRANSFORM_DOTPREFIX].Transform = TransformDotPrefix;
    sigmatch_table[DETECT_TRANSFORM_DOTPREFIX].Setup = DetectTransformDotPrefixSetup;
    sigmatch_table[DETECT_TRANSFORM_DOTPREFIX].Free = DetectTransformDotPrefixFree;
    sigmatch_table[DETECT_TRANSFORM_DOTPREFIX].RegisterTests =
        DetectTransformDotPrefixRegisterTests;

    sigmatch_table[DETECT_TRANSFORM_DOTPREFIX].flags |= SIGMATCH_NOOPT;
}

/* Example -- to be removed before final pr. Transforms that supply
 * options implement Free. This function is only called when the options
 * value is non-null
 */
static void DetectTransformDotPrefixFree(void *ptr)
{
    SCLogNotice("Entering %s with %p", __FUNCTION__, ptr);
    if (ptr)
        SCFree(ptr);
}
/**
 *  \internal
 *  \brief Extract the dotprefix, if any, the last pattern match, either content or uricontent
 *  \param det_ctx detection engine ctx
 *  \param s signature
 *  \param nullstr should be null
 *  \retval 0 ok
 *  \retval -1 failure
 */
static int DetectTransformDotPrefixSetup (DetectEngineCtx *de_ctx, Signature *s, const char *nullstr)
{
    SCEnter();
    /* Example: to be removed from the final pr. This exemplifies what happens if a transform
     * detects options. The detection logic is TBD and will likely be `transform:option-values`
     */
    char *options = SCMalloc(10);
    if (options == NULL) {
        SCLogNotice("Unable to allocate memory for options structure");
    } else {
        SCLogNotice("%s allocated %p", __FUNCTION__, options);
    }
    int r = DetectSignatureAddTransform(s, DETECT_TRANSFORM_DOTPREFIX, options);
    SCReturnInt(r);
}

/**
 * \brief Return the dotprefix, if any, in the last pattern match.
 *
 * Input values are modified by prefixing with a ".".
 *
 * Rule: "alert dns any any -> any any (dns_query; dotprefix; content:".google.com"; sid:1;)"
 * 1. hello.google.com --> match
 * 2. hey.agoogle.com --> no match
 * 3. agoogle.com --> no match
 * 4. something.google.com.au --> match
 * 5. google.com --> match
 *
 * To match on the dotprefix only:
 * Rule: "alert dns any any -> any any (dns_query; dotprefix; content:".google.com"; endswith; sid:1;)"
 *
 * 1. hello.google.com --> match
 * 2. hey.agoogle.com --> no match
 * 3. agoogle.com --> no match
 * 4. something.google.com.au --> no match
 * 5. google.com --> match
 *
 * To match on a TLD:
 * Rule: "alert dns any any -> any any (dns_query; dotprefix; content:".co.uk"; endswith; sid:1;)"
 *
 * 1. hello.google.com --> no match
 * 2. hey.agoogle.com --> no match
 * 3. agoogle.com --> no match
 * 4. something.google.co.uk --> match
 * 5. google.com --> no match
 */
static void TransformDotPrefix(InspectionBuffer *buffer, void *options)
{
    const size_t input_len = buffer->inspect_len;

    if (input_len) {
        uint8_t output[input_len + 1]; // For the leading '.'

        output[0] = '.';
        memcpy(&output[1], buffer->inspect, input_len);
        InspectionBufferCopy(buffer, output, input_len + 1);
    }
}

#ifdef UNITTESTS
static int DetectTransformDotPrefixTest01(void)
{
    const uint8_t *input = (const uint8_t *)"example.com";
    uint32_t input_len = strlen((char *)input);

    const char *result = ".example.com";
    uint32_t result_len = strlen((char *)result);

    InspectionBuffer buffer;
    InspectionBufferInit(&buffer, input_len);
    InspectionBufferSetup(&buffer, input, input_len);
    PrintRawDataFp(stdout, buffer.inspect, buffer.inspect_len);
    TransformDotPrefix(&buffer, NULL);
    PrintRawDataFp(stdout, buffer.inspect, buffer.inspect_len);
    FAIL_IF_NOT(buffer.inspect_len == result_len);
    FAIL_IF_NOT(strncmp(result, (const char *)buffer.inspect, result_len) == 0);
    InspectionBufferFree(&buffer);
    PASS;
}

static int DetectTransformDotPrefixTest02(void)
{
    const uint8_t *input = (const uint8_t *)"hello.example.com";
    uint32_t input_len = strlen((char *)input);

    const char *result = ".hello.example.com";
    uint32_t result_len = strlen((char *)result);

    InspectionBuffer buffer;
    InspectionBufferInit(&buffer, input_len);
    InspectionBufferSetup(&buffer, input, input_len);
    PrintRawDataFp(stdout, buffer.inspect, buffer.inspect_len);
    TransformDotPrefix(&buffer, NULL);
    PrintRawDataFp(stdout, buffer.inspect, buffer.inspect_len);
    FAIL_IF_NOT(buffer.inspect_len == result_len);
    FAIL_IF_NOT(strncmp(result, (const char *)buffer.inspect, result_len) == 0);
    InspectionBufferFree(&buffer);
    PASS;
}

static int DetectTransformDotPrefixTest03(void)
{
    const char rule[] = "alert dns any any -> any any (dns.query; dotprefix; content:\".google.com\"; sid:1;)";
    ThreadVars th_v;
    DetectEngineThreadCtx *det_ctx = NULL;
    memset(&th_v, 0, sizeof(th_v));

    DetectEngineCtx *de_ctx = DetectEngineCtxInit();
    FAIL_IF_NULL(de_ctx);
    Signature *s = DetectEngineAppendSig(de_ctx, rule);
    FAIL_IF_NULL(s);
    SigGroupBuild(de_ctx);
    DetectEngineThreadCtxInit(&th_v, (void *)de_ctx, (void *)&det_ctx);
    DetectEngineThreadCtxDeinit(&th_v, (void *)det_ctx);
    DetectEngineCtxFree(de_ctx);
    PASS;
}
#endif

static void DetectTransformDotPrefixRegisterTests(void)
{
#ifdef UNITTESTS
    UtRegisterTest("DetectTransformDotPrefixTest01", DetectTransformDotPrefixTest01);
    UtRegisterTest("DetectTransformDotPrefixTest02", DetectTransformDotPrefixTest02);
    UtRegisterTest("DetectTransformDotPrefixTest03", DetectTransformDotPrefixTest03);
#endif
}
