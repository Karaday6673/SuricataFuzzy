/* Copyright (C) 2012 BAE Systems
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
 * \author David Abarbanel <david.abarbanel@baesystems.com>
 *
 */

#ifndef MIME_DECODE_H_
#define MIME_DECODE_H_

#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>

#include "suricata.h"
#include "util-debug.h"

/* Header Flags */

/* Content Flags */
#define CTNT_IS_MSG           1
#define CTNT_IS_ENV           2
#define CTNT_IS_ENCAP         4
#define CTNT_IS_BODYPART      8
#define CTNT_IS_MULTIPART    16
#define CTNT_IS_ATTACHMENT   32
#define CTNT_IS_BASE64       64
#define CTNT_IS_QP          128
#define CTNT_IS_TEXT        256
#define CTNT_IS_HTML        512

/* URL Flags */
#define URL_IS_IP          1
#define URL_IS_EXE         2
#define URL_IS_INVALID_IP  4

/* Anomaly Flags */
#define ANOM_INVALID_BASE64      1  /* invalid base64 chars */
#define ANOM_INVALID_QP          2  /* invalid qouted-printable chars */
#define ANOM_LONG_HEADER_NAME    4  /* header is abnormally long */
#define ANOM_LONG_HEADER_VALUE   8  /* header value is abnormally long
                                     * (includes multi-line) */
#define ANOM_LONG_LINE          16  /* Lines that exceed 998 octets */
#define ANOM_LONG_ENC_LINE      32  /* Lines that exceed 76 octets */
#define ANOM_MALFORMED_MSG      64  /* Misc msg format errors found */

/* Pubicly exposed size constants */
#define DATA_CHUNK_SIZE  3072  /* Should be divisible by 3 */
#define B64_BLOCK           4
#define LINEREM_SIZE      256

/* Mime Parser Constants */
#define HEADER_READY    0x01
#define HEADER_STARTED  0x02
#define HEADER_DONE     0x03
#define BODY_STARTED    0x04
#define BODY_DONE       0x05
#define BODY_END_BOUND  0x06
#define PARSE_DONE      0x07
#define PARSE_ERROR     0x08

/**
 * \brief Mime Decoder Error Codes
 */
typedef enum MimeDecRetCode {
    MIME_DEC_OK = 0,
    MIME_DEC_MORE = 1,
    MIME_DEC_ERR_DATA = -1,
    MIME_DEC_ERR_MEM = -2,
    MIME_DEC_ERR_PARSE = -3
} MimeDecRetCode;

/**
 * \brief Structure for containing configuration options
 *
 */
typedef struct MimeDecConfig {
    int decode_base64;  /**< Decode base64 bodies */
    int decode_quoted_printable;  /**< Decode quoted-printable bodies */
    int extract_urls;  /**< Extract and store URLs in data structure */
    uint32_t header_value_depth;  /**< Depth of which to store header values
                                       (Default is 2000) */
} MimeDecConfig;

/**
 * \brief This represents a header field name and associated value
 */
typedef struct MimeDecField {
    char *name;  /**< Name of the header field */
    uint32_t name_len;  /**< Length of the name */
    char *value;  /**< Value of the header field */
    uint32_t value_len;  /**< Length of the value */
    struct MimeDecField *next;  /**< Pointer to next field */
} MimeDecField;

/**
 * \brief This represents a URL value node in a linked list
 *
 * Since HTML can sometimes contain a high number of URLs, this
 * structure only features the URL host name/IP or those that are
 * pointing to an executable file (see url_flags to determine which).
 */
typedef struct MimeDecUrl {
    char *url;  /**< String representation of full or partial URL */
    uint32_t url_len;  /**< Length of the URL string */
    uint32_t url_flags;  /**< Flags indicating type of URL */
    uint32_t url_cnt;  /**< Count of URLs with same value */
    struct MimeDecUrl *next;  /**< Pointer to next URL */
} MimeDecUrl;

/**
 * \brief This represents the MIME Entity (or also top level message) in a
 * child-sibling tree
 */
typedef struct MimeDecEntity {
    MimeDecField *field_list;  /**< Pointer to list of header fields */
    MimeDecUrl *url_list;  /**< Pointer to list of URLs */
    uint32_t body_len;  /**< Length of body (prior to any decoding) */
    uint32_t decoded_body_len;  /**< Length of body after decoding */
    uint32_t header_flags; /**< Flags indicating header characteristics */
    uint32_t ctnt_flags;  /**< Flags indicating type of content */
    uint32_t anomaly_flags;  /**< Flags indicating an anomaly in the message */
    char *filename;  /**< Name of file attachment */
    uint32_t filename_len;  /**< Length of file attachment name */
    char *ctnt_type;  /**< Quick access pointer to short-hand content type field */
    uint32_t ctnt_type_len;  /**< Length of content type field value */
    char *msg_id;  /**< Quick access pointer to message Id */
    uint32_t msg_id_len;  /**< Quick access pointer to message Id */
    struct MimeDecEntity *next;  /**< Pointer to list of sibling entities */
    struct MimeDecEntity *child;  /**< Pointer to list of child entities */
} MimeDecEntity;

/**
 * \brief Structure contains boundary and entity for the current node (entity)
 * in the stack
 *
 */
typedef struct MimeDecStackNode {
    MimeDecEntity *data;  /**< Pointer to the entity data structure */
    char *bdef;  /**< Copy of boundary definition for child entity */
    uint32_t bdef_len;  /**< Boundary length for child entity */
    int is_encap;  /**< Flag indicating entity is encapsulated in message */
    struct MimeDecStackNode *next;  /**< Pointer to next item on the stack */
} MimeDecStackNode;

/**
 * \brief Structure holds the top of the stack along with some free reusable nodes
 *
 */
typedef struct MimeDecStack {
    MimeDecStackNode *top;  /**< Pointer to the top of the stack */
    MimeDecStackNode *free_nodes;  /**< Pointer to the list of free nodes */
    uint32_t free_nodes_cnt;  /**< Count of free nodes in the list */
} MimeDecStack;

/**
 * \brief Structure contains a list of value and lengths for robust data processing
 *
 */
typedef struct DataValue {
    char *value;  /**< Copy of data value */
    uint32_t value_len;  /**< Length of data value */
    struct DataValue *next;  /**< Pointer to next value in the list */
} DataValue;

/**
 * \brief Structure contains the current state of the MIME parser
 *
 */
typedef struct MimeDecParseState {
    MimeDecEntity *msg;  /**< Pointer to the top-level message entity */
    MimeDecStack *stack;  /**< Pointer to the top of the entity stack */
    char *hname;  /**< Copy of the last known header name */
    uint32_t hlen;  /**< Length of the last known header name */
    DataValue *hvalue;  /**< Pointer to the incomplete header value list */
    uint32_t hvlen; /**< Total length of value list */
    char linerem[LINEREM_SIZE];  /**< Remainder from previous line (for URL extraction) */
    uint16_t linerem_len;  /**< Length of remainder from previous line */
    char bvremain[B64_BLOCK];  /**< Remainder from base64-decoded line */
    uint8_t bvr_len;  /**< Length of remainder from base64-decoded line */
    uint8_t data_chunk[DATA_CHUNK_SIZE];  /**< Buffer holding data chunk */
    uint32_t data_chunk_len;  /**< Length of data chunk */
    int found_child;  /**< Flag indicating a child entity was found */
    int body_begin;  /**< Currently at beginning of body */
    int body_end;  /**< Currently at end of body */
    uint8_t state_flag;  /**<  Flag representing current state of parser */
    void *data;  /**< Pointer to data specific to the caller */
    int (*dataChunkProcessor) (const uint8_t *chunk, uint32_t len,
            struct MimeDecParseState *state);  /**< Data chunk processing function callback */
} MimeDecParseState;

/* Config functions */
void MimeDecSetConfig(MimeDecConfig *config);
MimeDecConfig * MimeDecGetConfig(void);

/* Memory functions */
void MimeDecFreeEntity(MimeDecEntity *entity);
void MimeDecFreeField(MimeDecField *field);
void MimeDecFreeUrl(MimeDecUrl *url);

/* List functions */
MimeDecField * MimeDecAddField(MimeDecEntity *entity);
MimeDecField * MimeDecFindField(const MimeDecEntity *entity, const char *name);
MimeDecUrl * MimeDecAddUrl(MimeDecEntity *entity);
MimeDecEntity * MimeDecAddEntity(MimeDecEntity *parent);

/* Helper functions */
MimeDecField * MimeDecFillField(MimeDecEntity *entity, const char *name,
        uint32_t nlen, const char *value, uint32_t vlen, int copy_name_value);

/* Parser functions */
MimeDecParseState * MimeDecInitParser(void *data, int (*dcpfunc)(const uint8_t *chunk,
        uint32_t len, MimeDecParseState *state));
void MimeDecDeInitParser(MimeDecParseState *state);
int MimeDecParseComplete(MimeDecParseState *state);
int MimeDecParseLine(const char *line, const uint32_t len, MimeDecParseState *state);
MimeDecEntity * MimeDecParseFullMsg(const char *buf, uint32_t blen, void *data,
        int (*dcpfunc)(const uint8_t *chunk, uint32_t len, MimeDecParseState *state));

/* Test functions */
void MimeDecRegisterTests(void);

#endif
