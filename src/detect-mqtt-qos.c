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
#include "detect-mqtt-qos.h"
#include "util-unittest.h"

#include "rust-bindings.h"

#define PARSE_REGEX "^\\s*[012]$"
static DetectParseRegex parse_regex;

static int mqtt_qos_id = 0;

static int DetectMQTTQosMatch(DetectEngineThreadCtx *det_ctx,
                               Flow *f, uint8_t flags, void *state,
                               void *txv, const Signature *s,
                               const SigMatchCtx *ctx);
static int DetectMQTTQosSetup (DetectEngineCtx *, Signature *, const char *);
void MQTTQosRegisterTests(void);
void DetectMQTTQosFree(void *);

static int DetectEngineInspectMQTTQosGeneric(ThreadVars *tv,
        DetectEngineCtx *de_ctx, DetectEngineThreadCtx *det_ctx,
        const Signature *s, const SigMatchData *smd,
        Flow *f, uint8_t flags, void *alstate,
        void *txv, uint64_t tx_id);

typedef struct DetectMQTTQosData_ {
    uint8_t qos;
} DetectMQTTQosData;

/**
 * \brief Registration function for mqtt.qos: keyword
 */
void DetectMQTTQosRegister (void)
{
    sigmatch_table[DETECT_AL_MQTT_QOS].name = "mqtt.qos";
    sigmatch_table[DETECT_AL_MQTT_QOS].desc = "match MQTT fixed header QOS level";
    sigmatch_table[DETECT_AL_MQTT_QOS].url = DOC_URL DOC_VERSION "/rules/mqtt-keywords.html#mqtt-qos";
    sigmatch_table[DETECT_AL_MQTT_QOS].AppLayerTxMatch = DetectMQTTQosMatch;
    sigmatch_table[DETECT_AL_MQTT_QOS].Setup = DetectMQTTQosSetup;
    sigmatch_table[DETECT_AL_MQTT_QOS].Free  = DetectMQTTQosFree;
    sigmatch_table[DETECT_AL_MQTT_QOS].RegisterTests = MQTTQosRegisterTests;

    DetectSetupParseRegexes(PARSE_REGEX, &parse_regex);

    DetectAppLayerInspectEngineRegister("mqtt.qos",
            ALPROTO_MQTT, SIG_FLAG_TOSERVER, 1,
            DetectEngineInspectMQTTQosGeneric);

    mqtt_qos_id = DetectBufferTypeGetByName("mqtt.qos");
}

static int DetectEngineInspectMQTTQosGeneric(ThreadVars *tv,
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
 * \brief Function to match fixed header QOS field of an MQTT Tx
 *
 * \param det_ctx Pointer to the pattern matcher thread.
 * \param f       Pointer to the current flow.
 * \param flags   Flags.
 * \param state   App layer state.
 * \param txv     Pointer to the transaction.
 * \param s       Pointer to the Signature.
 * \param ctx     Pointer to the sigmatch that we will cast into DetectMQTTQosData.
 *
 * \retval 0 no match.
 * \retval 1 match.
 */
static int DetectMQTTQosMatch(DetectEngineThreadCtx *det_ctx,
                               Flow *f, uint8_t flags, void *state,
                               void *txv, const Signature *s,
                               const SigMatchCtx *ctx)
{
    const DetectMQTTQosData *de = (const DetectMQTTQosData *)ctx;
    uint8_t qosval;

    if (!de)
        return 0;

    rs_mqtt_tx_get_qos(txv, &qosval);
    if (qosval == de->qos)
        return 1;

    return 0;
}

/**
 * \internal
 * \brief This function is used to parse options passed via mqtt.qos: keyword
 *
 * \param rawstr Pointer to the user provided options
 *
 * \retval de pointer to DetectMQTTQosData on success
 * \retval NULL on failure
 */
static DetectMQTTQosData *DetectMQTTQosParse(const char *rawstr)
{
    DetectMQTTQosData *de = NULL;
#define MAX_SUBSTRINGS 30
    int ret = 0;
    uint8_t val;
    int ov[MAX_SUBSTRINGS];

    ret = DetectParsePcreExec(&parse_regex, rawstr, 0, 0, ov, MAX_SUBSTRINGS);
    if (ret < 1) {
        SCLogError(SC_ERR_PCRE_MATCH, "invalid MQTT QOS level: %s", rawstr);
        return NULL;
    }

    ret = sscanf(rawstr, "%hhd", &val);
    if (ret != 1) {
        SCLogError(SC_ERR_UNKNOWN_VALUE, "invalid MQTT QOS level: %s", rawstr);
        return NULL;
    }

    de = SCMalloc(sizeof(DetectMQTTQosData));
    if (unlikely(de == NULL))
        return NULL;
    de->qos = val;

    return de;
}

/**
 * \internal
 * \brief this function is used to add the parsed sigmatch  into the current signature
 *
 * \param de_ctx pointer to the Detection Engine Context
 * \param s pointer to the Current Signature
 * \param rawstr pointer to the user provided options
 *
 * \retval 0 on Success
 * \retval -1 on Failure
 */
static int DetectMQTTQosSetup (DetectEngineCtx *de_ctx, Signature *s, const char *rawstr)
{
    DetectMQTTQosData *de = NULL;
    SigMatch *sm = NULL;

    de = DetectMQTTQosParse(rawstr);
    if (de == NULL)
        goto error;

    sm = SigMatchAlloc();
    if (sm == NULL)
        goto error;

    sm->type = DETECT_AL_MQTT_QOS;
    sm->ctx = (SigMatchCtx *)de;

    SigMatchAppendSMToList(s, sm, mqtt_qos_id);

    return 0;

error:
    if (de) SCFree(de);
    if (sm) SCFree(sm);
    return -1;
}

/**
 * \internal
 * \brief this function will free memory associated with DetectMQTTQosData
 *
 * \param de pointer to DetectMQTTQosData
 */
void DetectMQTTQosFree(void *de_ptr)
{
    DetectMQTTQosData *de = (DetectMQTTQosData *)de_ptr;
    if(de) SCFree(de);
}

/*
 * ONLY TESTS BELOW THIS COMMENT
 */

#ifdef UNITTESTS
/**
 * \test MQTTQosTestParse01 is a test for a valid value
 *
 *  \retval 1 on success
 *  \retval 0 on failure
 */
static int MQTTQosTestParse01 (void)
{
    DetectMQTTQosData *de = NULL;
    de = DetectMQTTQosParse("0");
    if (!de) {
        return 0;
    }
    DetectMQTTQosFree(de);
    de = DetectMQTTQosParse("   0");
    if (!de) {
        return 0;
    }
    DetectMQTTQosFree(de);
    de = DetectMQTTQosParse("1");
    if (!de) {
        return 0;
    }
    DetectMQTTQosFree(de);
    de = DetectMQTTQosParse("2");
    if (!de) {
        return 0;
    }
    DetectMQTTQosFree(de);

    return 1;
}

/**
 * \test MQTTQosTestParse02 is a test for an invalid value
 *
 *  \retval 1 on success
 *  \retval 0 on failure
 */
static int MQTTQosTestParse02 (void)
{
    DetectMQTTQosData *de = NULL;
    de = DetectMQTTQosParse("3");
    if (de) {
        DetectMQTTQosFree(de);
        return 0;
    }

    return 1;
}

/**
 * \test MQTTQosTestParse04 is a test for an invalid value
 *
 *  \retval 1 on success
 *  \retval 0 on failure
 */
static int MQTTQosTestParse03 (void)
{
    DetectMQTTQosData *de = NULL;
    de = DetectMQTTQosParse("12");
    if (de) {
        DetectMQTTQosFree(de);
        return 0;
    }

    return 1;
}


#endif /* UNITTESTS */

/**
 * \brief this function registers unit tests for MQTTQos
 */
void MQTTQosRegisterTests(void)
{
#ifdef UNITTESTS
    UtRegisterTest("MQTTQosTestParse01", MQTTQosTestParse01);
    UtRegisterTest("MQTTQosTestParse02", MQTTQosTestParse02);
    UtRegisterTest("MQTTQosTestParse03", MQTTQosTestParse03);
#endif /* UNITTESTS */
}