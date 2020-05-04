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
 * \author Sascha Steinbiss <sascha@steinbiss.name>
 */

#include "suricata-common.h"
#include "conf.h"
#include "detect.h"
#include "detect-parse.h"
#include "detect-engine.h"
#include "detect-engine-content-inspection.h"
#include "detect-mqtt-flags.h"
#include "util-unittest.h"

#include "rust-bindings.h"

#define PARSE_REGEX "(?: *,?!?(?:retain|dup))+"
static DetectParseRegex parse_regex;

static int mqtt_flags_id = 0;

static int DetectMQTTFlagsMatch(DetectEngineThreadCtx *det_ctx,
                               Flow *f, uint8_t flags, void *state,
                               void *txv, const Signature *s,
                               const SigMatchCtx *ctx);
static int DetectMQTTFlagsSetup (DetectEngineCtx *, Signature *, const char *);
void MQTTFlagsRegisterTests(void);
void DetectMQTTFlagsFree(DetectEngineCtx *de_ctx, void *);

static int DetectEngineInspectMQTTFlagsGeneric(ThreadVars *tv,
        DetectEngineCtx *de_ctx, DetectEngineThreadCtx *det_ctx,
        const Signature *s, const SigMatchData *smd,
        Flow *f, uint8_t flags, void *alstate,
        void *txv, uint64_t tx_id);

typedef struct DetectMQTTFlagsData_ {
    MQTTFlagState retain, dup;
} DetectMQTTFlagsData;

/**
 * \brief Registration function for mqtt.flags: keyword
 */
void DetectMQTTFlagsRegister (void)
{
    sigmatch_table[DETECT_AL_MQTT_FLAGS].name = "mqtt.flags";
    sigmatch_table[DETECT_AL_MQTT_FLAGS].desc = "match MQTT fixed header flags";
    sigmatch_table[DETECT_AL_MQTT_FLAGS].url = "/rules/mqtt-keywords.html#mqtt-flags";
    sigmatch_table[DETECT_AL_MQTT_FLAGS].AppLayerTxMatch = DetectMQTTFlagsMatch;
    sigmatch_table[DETECT_AL_MQTT_FLAGS].Setup = DetectMQTTFlagsSetup;
    sigmatch_table[DETECT_AL_MQTT_FLAGS].Free  = DetectMQTTFlagsFree;
    sigmatch_table[DETECT_AL_MQTT_FLAGS].RegisterTests = MQTTFlagsRegisterTests;

    DetectSetupParseRegexes(PARSE_REGEX, &parse_regex);

    DetectAppLayerInspectEngineRegister("mqtt.flags",
            ALPROTO_MQTT, SIG_FLAG_TOSERVER, 1,
            DetectEngineInspectMQTTFlagsGeneric);

    mqtt_flags_id = DetectBufferTypeGetByName("mqtt.flags");
}

static int DetectEngineInspectMQTTFlagsGeneric(ThreadVars *tv,
        DetectEngineCtx *de_ctx, DetectEngineThreadCtx *det_ctx,
        const Signature *s, const SigMatchData *smd,
        Flow *f, uint8_t flags, void *alstate,
        void *txv, uint64_t tx_id)
{
    return DetectEngineInspectGenericList(tv, de_ctx, det_ctx, s, smd,
                                          f, flags, alstate, txv, tx_id);
}

/**
 * \internal
 * \brief Function to match fixed header flags of an MQTT Tx
 *
 * \param det_ctx Pointer to the pattern matcher thread.
 * \param f       Pointer to the current flow.
 * \param flags   Flags.
 * \param state   App layer state.
 * \param txv     Pointer to the transaction.
 * \param s       Pointer to the Signature.
 * \param ctx     Pointer to the sigmatch that we will cast into DetectMQTTFlagsData.
 *
 * \retval 0 no match.
 * \retval 1 match.
 */
static int DetectMQTTFlagsMatch(DetectEngineThreadCtx *det_ctx,
                               Flow *f, uint8_t flags, void *state,
                               void *txv, const Signature *s,
                               const SigMatchCtx *ctx)
{
    const DetectMQTTFlagsData *de = (const DetectMQTTFlagsData *)ctx;

    if (!de)
        return 0;

    return rs_mqtt_tx_has_flags(txv, de->retain, de->dup);
}

/**
 * \internal
 * \brief This function is used to parse options passed via mqtt.flags: keyword
 *
 * \param rawstr Pointer to the user provided options
 *
 * \retval de pointer to DetectMQTTFlagsData on success
 * \retval NULL on failure
 */
static DetectMQTTFlagsData *DetectMQTTFlagsParse(const char *rawstr)
{
    DetectMQTTFlagsData *de = NULL;
#define MAX_SUBSTRINGS 30
    int ret = 0;
    int ov[MAX_SUBSTRINGS];

    ret = DetectParsePcreExec(&parse_regex, rawstr, 0, 0, ov, MAX_SUBSTRINGS);
    if (ret < 1) {
        SCLogError(SC_ERR_PCRE_MATCH, "invalid flag definition: %s", rawstr);
        return NULL;
    }

    de = SCMalloc(sizeof(DetectMQTTFlagsData));
    if (unlikely(de == NULL))
        return NULL;
    de->retain = de->dup = DONT_CARE;

    char copy[strlen(rawstr)+1];
    strlcpy(copy, rawstr, sizeof(copy));
    char *xsaveptr = NULL;

    /* Iterate through comma-separated string... */
    char *flagv = strtok_r(copy, ",", &xsaveptr);
    while (flagv != NULL) {
        /* skip blanks */
        while (*flagv != '\0' && isblank(*flagv)) {
            flagv++;
        }
        if (strlen(flagv) < 2) {
            /* flags have a minimum length */
            SCLogError(SC_ERR_UNKNOWN_VALUE, "malformed flag value: %s", flagv);
            goto error;
        }  else {
            if (flagv[0] == '!') {
                /* handle negated flag */
                if (strcmp(flagv+1, "dup") == 0) {
                    if (de->dup != DONT_CARE) {
                        SCLogError(SC_ERR_INVALID_VALUE, "duplicate flag definition: %s", flagv);
                        goto error;
                    }
                    de->dup = CANT_BE_SET;
                } else if (strcmp(flagv+1, "retain") == 0) {
                    if (de->retain != DONT_CARE) {
                        SCLogError(SC_ERR_INVALID_VALUE, "duplicate flag definition: %s", flagv);
                        goto error;
                    }
                    de->retain = CANT_BE_SET;
                } else {
                    SCLogError(SC_ERR_UNKNOWN_VALUE, "invalid flag definition: %s", flagv);
                    goto error;
                }
            } else {
                /* handle non-negated flag */
                if (strcmp(flagv, "dup") == 0) {
                    if (de->dup != DONT_CARE) {
                        SCLogError(SC_ERR_INVALID_VALUE, "duplicate flag definition: %s", flagv);
                        goto error;
                    }
                    de->dup = MUST_BE_SET;
                } else if (strcmp(flagv, "retain") == 0) {
                    if (de->retain != DONT_CARE) {
                        SCLogError(SC_ERR_INVALID_VALUE, "duplicate flag definition: %s", flagv);
                        goto error;
                    }
                    de->retain = MUST_BE_SET;
                } else {
                    SCLogError(SC_ERR_UNKNOWN_VALUE, "invalid flag definition: %s", flagv);
                    goto error;
                }
            }
        }
        flagv = strtok_r(NULL, ",", &xsaveptr);
    }

    return de;

error:
    SCFree(de);
    return NULL;
}

/**
 * \internal
 * \brief this function is used to add the parsed type query into the current signature
 *
 * \param de_ctx pointer to the Detection Engine Context
 * \param s pointer to the Current Signature
 * \param rawstr pointer to the user provided options
 *
 * \retval 0 on Success
 * \retval -1 on Failure
 */
static int DetectMQTTFlagsSetup(DetectEngineCtx *de_ctx, Signature *s, const char *rawstr)
{
    DetectMQTTFlagsData *de = NULL;
    SigMatch *sm = NULL;

    de = DetectMQTTFlagsParse(rawstr);
    if (de == NULL)
        goto error;

    sm = SigMatchAlloc();
    if (sm == NULL)
        goto error;

    sm->type = DETECT_AL_MQTT_FLAGS;
    sm->ctx = (SigMatchCtx *)de;

    SigMatchAppendSMToList(s, sm, mqtt_flags_id);

    return 0;

error:
    SCFree(de);
    SCFree(sm);
    return -1;
}

/**
 * \internal
 * \brief this function will free memory associated with DetectMQTTFlagsData
 *
 * \param de pointer to DetectMQTTFlagsData
 */
void DetectMQTTFlagsFree(DetectEngineCtx *de_ctx, void *de_ptr)
{
    DetectMQTTFlagsData *de = (DetectMQTTFlagsData *)de_ptr;
    SCFree(de);
}

/*
 * ONLY TESTS BELOW THIS COMMENT
 */

#ifdef UNITTESTS
/**
 * \test MQTTFlagsTestParse01 is a test for a valid value
 *
 *  \retval 1 on success
 *  \retval 0 on failure
 */
static int MQTTFlagsTestParse01 (void)
{
    DetectMQTTFlagsData *de = NULL;

    de = DetectMQTTFlagsParse("retain");
    FAIL_IF_NULL(de);
    DetectMQTTFlagsFree(NULL, de);

    de = DetectMQTTFlagsParse("dup");
    FAIL_IF_NULL(de);
    DetectMQTTFlagsFree(NULL, de);

    de = DetectMQTTFlagsParse("retain,dup");
    FAIL_IF_NULL(de);
    DetectMQTTFlagsFree(NULL, de);

    de = DetectMQTTFlagsParse("dup, retain");
    FAIL_IF_NULL(de);
    DetectMQTTFlagsFree(NULL, de);

    PASS;
}

/**
 * \test MQTTFlagsTestParse02 is a test for a valid value
 *
 *  \retval 1 on success
 *  \retval 0 on failure
 */
static int MQTTFlagsTestParse02 (void)
{
    DetectMQTTFlagsData *de = NULL;
    de = DetectMQTTFlagsParse("retain,!dup");
    FAIL_IF_NULL(de);
    DetectMQTTFlagsFree(NULL, de);

    PASS;
}

/**
 * \test MQTTFlagsTestParse03 is a test for an invalid value
 *
 *  \retval 1 on success
 *  \retval 0 on failure
 */
static int MQTTFlagsTestParse03 (void)
{
    DetectMQTTFlagsData *de = NULL;
    de = DetectMQTTFlagsParse("ref");
    if (de) {
        DetectMQTTFlagsFree(NULL, de);
        FAIL;
    }

    PASS;
}

/**
 * \test MQTTFlagsTestParse04 is a test for an invalid value
 *
 *  \retval 1 on success
 *  \retval 0 on failure
 */
static int MQTTFlagsTestParse04 (void)
{
    DetectMQTTFlagsData *de = NULL;
    de = DetectMQTTFlagsParse("dup,!");
    if (de) {
        DetectMQTTFlagsFree(NULL, de);
        FAIL;
    }

    PASS;
}

/**
 * \test MQTTFlagsTestParse05 is a test for an invalid value
 *
 *  \retval 1 on success
 *  \retval 0 on failure
 */
static int MQTTFlagsTestParse05 (void)
{
    DetectMQTTFlagsData *de = NULL;
    de = DetectMQTTFlagsParse("dup,!dup");
    if (de) {
        DetectMQTTFlagsFree(NULL, de);
        FAIL;
    }

    de = DetectMQTTFlagsParse("!retain,retain");
    if (de) {
        DetectMQTTFlagsFree(NULL, de);
        FAIL;
    }

    PASS;
}


#endif /* UNITTESTS */

/**
 * \brief this function registers unit tests for MQTTFlags
 */
void MQTTFlagsRegisterTests(void)
{
#ifdef UNITTESTS
    UtRegisterTest("MQTTFlagsTestParse01", MQTTFlagsTestParse01);
    UtRegisterTest("MQTTFlagsTestParse02", MQTTFlagsTestParse02);
    UtRegisterTest("MQTTFlagsTestParse03", MQTTFlagsTestParse03);
    UtRegisterTest("MQTTFlagsTestParse04", MQTTFlagsTestParse04);
    UtRegisterTest("MQTTFlagsTestParse05", MQTTFlagsTestParse05);
#endif /* UNITTESTS */
}