/* Copyright (C) 2007-2016 Open Information Security Foundation
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
 * \author Duarte Silva <duarte.silva@serializing.me>
 *
 */

#include "suricata-common.h"

#include "detect.h"
#include "detect-parse.h"

#include "util-detect-file-hash.h"

#include "app-layer-htp.h"

#ifdef HAVE_NSS

/**
 * \brief Read the bytes of a hash from an hexadecimal string
 *
 * \param hash buffer to store the resulting bytes
 * \param string hexadecimal string representing the hash
 * \param filename file name from where the string was read
 * \param line_no file line number from where the string was read
 * \param expected_len the expected length of the string that was read
 *
 * \retval -1 the hexadecimal string is invalid
 * \retval 1 the hexadecimal string was read successfully
 */
int ReadHashString(uint8_t *hash, char *string, char *filename, int line_no,
        uint16_t expected_len)
{
    if (strlen(string) != expected_len) {
        SCLogError(SC_ERR_INVALID_HASH, "%s:%d hash string not %d characters",
                filename, line_no, expected_len);
        return -1;
    }

    int i, x;
    for (x = 0, i = 0; i < expected_len; i+=2, x++) {
        char buf[3] = { 0, 0, 0 };
        buf[0] = string[i];
        buf[1] = string[i+1];

        long value = strtol(buf, NULL, 16);
        if (value >= 0 && value <= 255)
            hash[x] = (uint8_t)value;
        else {
            SCLogError(SC_ERR_INVALID_HASH, "%s:%d hash byte out of range %ld",
                    filename, line_no, value);
            return -1;
        }
    }

    return 1;
}

/**
 * \brief Store a hash into the hash table
 *
 * \param hash_table hash table that will hold the hash
 * \param string hexadecimal string representing the hash
 * \param filename file name from where the string was read
 * \param line_no file line number from where the string was read
 * \param type the hash algorithm
 *
 * \retval -1 failed to load the hash into the hash table
 * \retval 1 successfully loaded the has into the hash table
 */
int LoadHashTable(ROHashTable *hash_table, char *string, char *filename,
        int line_no, uint32_t type)
{
    /* allocate the maximum size a hash can have (in this case is SHA256, 32 bytes) */
    uint8_t hash[32];
    /* specify the actual size that should be read depending on the hash algorithm */
    uint16_t size = 32;

    if (type == DETECT_FILEMD5) {
        size = 16;
    }
    else if (type == DETECT_FILESHA1) {
        size = 20;
    }

    /* every byte represented with hexadecimal digits is two characters */
    uint16_t expected_len = (size * 2);

    if (ReadHashString(hash, string, filename, line_no, expected_len) == 1) {
        if (ROHashInitQueueValue(hash_table, &hash, size) != 1)
            return -1;
    }

    return 1;
}

/**
 * \brief Match a hash stored in a hash table
 *
 * \param hash_table hash table that will hold the hash
 * \param hash buffer containing the bytes of the has
 * \param hash_len length of the hash buffer
 *
 * \retval 0 didn't find the specified hash
 * \retval 1 the hash matched a stored value
 */
static int HashMatchHashTable(ROHashTable *hash_table, uint8_t *hash,
        size_t hash_len)
{
    void *ptr = ROHashLookup(hash_table, hash, (uint16_t)hash_len);
    if (ptr == NULL)
        return 0;
    else
        return 1;
}

/**
 * \brief Match the specified file hash
 *
 * \param t thread local vars
 * \param det_ctx pattern matcher thread local data
 * \param f *LOCKED* flow
 * \param flags direction flags
 * \param file file being inspected
 * \param s signature being inspected
 * \param m sigmatch that we will cast into DetectFileHashData
 *
 * \retval 0 no match
 * \retval 1 match
 */
int DetectFileHashMatch (ThreadVars *t, DetectEngineThreadCtx *det_ctx,
        Flow *f, uint8_t flags, File *file, Signature *s, SigMatch *m)
{
    SCEnter();
    int ret = 0;
    DetectFileHashData *filehash = (DetectFileHashData *)m->ctx;

    if (file->txid < det_ctx->tx_id) {
        SCReturnInt(0);
    }

    if (file->txid > det_ctx->tx_id) {
        SCReturnInt(0);
    }

    if (file->state != FILE_STATE_CLOSED) {
        SCReturnInt(0);
    }

    int match = -1;

    if (s->file_flags & FILE_SIG_NEED_MD5 && file->flags & FILE_MD5) {
        match = HashMatchHashTable(filehash->hash, file->md5, sizeof(file->md5));
    }
    else if (s->file_flags & FILE_SIG_NEED_SHA1 && file->flags & FILE_SHA1) {
        match = HashMatchHashTable(filehash->hash, file->sha1, sizeof(file->sha1));
    }
    else if (s->file_flags & FILE_SIG_NEED_SHA256 && file->flags & FILE_SHA256) {
        match = HashMatchHashTable(filehash->hash, file->sha256, sizeof(file->sha256));
    }

    if (match == 1) {
        if (filehash->negated == 0)
            ret = 1;
        else
            ret = 0;
    }
    else if (match == 0) {
        if (filehash->negated == 0)
            ret = 0;
        else
            ret = 1;
    }

    SCReturnInt(ret);
}

/**
 * \brief Parse the filemd5, filesha1 or filesha256 keyword
 *
 * \param det_ctx pattern matcher thread local data
 * \param str Pointer to the user provided option
 * \param type the hash algorithm
 *
 * \retval hash pointer to DetectFileHashData on success
 * \retval NULL on failure
 */
static DetectFileHashData *DetectFileHashParse (const DetectEngineCtx *de_ctx,
        char *str, uint32_t type)
{
    DetectFileHashData *filehash = NULL;
    FILE *fp = NULL;
    char *filename = NULL;

    /* We have a correct hash algorithm option */
    filehash = SCMalloc(sizeof(DetectFileHashData));
    if (unlikely(filehash == NULL))
        goto error;

    memset(filehash, 0x00, sizeof(DetectFileHashData));

    if (strlen(str) && str[0] == '!') {
        filehash->negated = 1;
        str++;
    }

    if (type == DETECT_FILEMD5) {
        filehash->hash = ROHashInit(18, 16);
    }
    else if (type == DETECT_FILESHA1) {
        filehash->hash = ROHashInit(18, 20);
    }
    else if (type == DETECT_FILESHA256) {
        filehash->hash = ROHashInit(18, 32);
    }

    if (filehash->hash == NULL) {
        goto error;
    }

    /* get full filename */
    filename = DetectLoadCompleteSigPath(de_ctx, str);
    if (filename == NULL) {
        goto error;
    }

    char line[8192] = "";
    fp = fopen(filename, "r");
    if (fp == NULL) {
        SCLogError(SC_ERR_OPENING_RULE_FILE, "opening hash file %s: %s", filename, strerror(errno));
        goto error;
    }

    int line_no = 0;
    while(fgets(line, (int)sizeof(line), fp) != NULL) {
        size_t len = strlen(line);
        line_no++;

        /* ignore comments and empty lines */
        if (line[0] == '\n' || line [0] == '\r' || line[0] == ' ' || line[0] == '#' || line[0] == '\t')
            continue;

        while (isspace(line[--len]));

        /* Check if we have a trailing newline, and remove it */
        len = strlen(line);
        if (len > 0 && (line[len - 1] == '\n' || line[len - 1] == '\r')) {
            line[len - 1] = '\0';
        }

        /* cut off longer lines than a SHA256 represented in hexadecimal  */
        if (strlen(line) > 64)
            line[64] = 0x00;

        if (LoadHashTable(filehash->hash, line, filename, line_no, type) != 1) {
            goto error;
        }
    }
    fclose(fp);
    fp = NULL;

    if (ROHashInitFinalize(filehash->hash) != 1) {
        goto error;
    }
    SCLogInfo("Hash hash table size %u bytes%s", ROHashMemorySize(filehash->hash), filehash->negated ? ", negated match" : "");

    SCFree(filename);
    return filehash;

error:
    if (filehash != NULL)
        DetectFileHashFree(filehash);
    if (fp != NULL)
        fclose(fp);
    if (filename != NULL)
        SCFree(filename);
    return NULL;
}

/**
 * \brief this function is used to parse filemd5, filesha1 and filesha256 options
 * \brief into the current signature
 *
 * \param de_ctx pointer to the Detection Engine Context
 * \param s pointer to the Current Signature
 * \param str pointer to the user provided "filemd5", "filesha1" or "filesha256" option
 * \param type type of file hash to setup
 *
 * \retval 0 on Success
 * \retval -1 on Failure
 */
int DetectFileHashSetup (DetectEngineCtx *de_ctx, Signature *s, char *str,
        uint32_t type)
{
    DetectFileHashData *filehash = NULL;
    SigMatch *sm = NULL;

    filehash = DetectFileHashParse(de_ctx, str, type);
    if (filehash == NULL)
        goto error;

    /* Okay so far so good, lets get this into a SigMatch
     * and put it in the Signature. */
    sm = SigMatchAlloc();
    if (sm == NULL)
        goto error;

    sm->type = type;
    sm->ctx = (void *)filehash;

    SigMatchAppendSMToList(s, sm, DETECT_SM_LIST_FILEMATCH);

    if (s->alproto != ALPROTO_HTTP && s->alproto != ALPROTO_SMTP) {
        SCLogError(SC_ERR_CONFLICTING_RULE_KEYWORDS, "rule contains conflicting keywords.");
        goto error;
    }

    if (s->alproto == ALPROTO_HTTP) {
        AppLayerHtpNeedFileInspection();
    }

    s->file_flags |= FILE_SIG_NEED_FILE;

    // Setup the file flags depending on the hashing algorithm
    if (type == DETECT_FILEMD5) {
        s->file_flags |= FILE_SIG_NEED_MD5;
    }
    if (type == DETECT_FILESHA1) {
        s->file_flags |= FILE_SIG_NEED_SHA1;
    }
    if (type == DETECT_FILESHA256) {
        s->file_flags |= FILE_SIG_NEED_SHA256;
    }
    return 0;

error:
    if (filehash != NULL)
        DetectFileHashFree(filehash);
    if (sm != NULL)
        SCFree(sm);
    return -1;
}

/**
 * \brief this function will free memory associated with DetectFileHashData
 *
 * \param filehash pointer to DetectFileHashData
 */
void DetectFileHashFree(void *ptr)
{
    if (ptr != NULL) {
        DetectFileHashData *filehash = (DetectFileHashData *)ptr;
        if (filehash->hash != NULL)
            ROHashFree(filehash->hash);
        SCFree(filehash);
    }
}

#endif /* HAVE_NSS */
