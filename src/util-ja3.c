/* Copyright (C) 2007-2017 Open Information Security Foundation
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
 * \author Mats Klepsland <mats.klepsland@gmail.com>
 *
 * Functions used to generate JA3 fingerprint.
 */

#include "suricata-common.h"

#include "util-ja3.h"

#define MD5_STRING_LENGTH 33

/**
 * \brief Allocate new buffer.
 *
 * \return pointer to buffer on success.
 * \return NULL on failure.
 */
JA3Buffer *Ja3BufferInit(void)
{
    JA3Buffer *buffer = SCCalloc(1, sizeof(JA3Buffer));
    if (buffer == NULL) {
        SCLogError(SC_ERR_MEM_ALLOC, "Error allocating memory for JA3 buffer");
        return NULL;
    }

    return buffer;
}

/**
 * \brief Free allocated buffer.
 *
 * \param buffer The buffer to free.
 */
void Ja3BufferFree(JA3Buffer *buffer)
{
    if (buffer == NULL) {
        SCLogError(SC_ERR_INVALID_ARGUMENT, "Buffer should not be NULL");
        return;
    }

    if (buffer->data != NULL) {
        SCFree(buffer->data);
    }

    SCFree(buffer);
}

/**
 * \internal
 * \brief Resize buffer if it is full.
 *
 * \param buffer The buffer.
 * \param len    The length of the data that should fit into the buffer.
 *
 * \retval 0 on success.
 * \retval -1 on failure.
 */
static int Ja3BufferResizeIfFull(JA3Buffer *buffer, uint32_t len)
{
    if (buffer == NULL) {
        SCLogError(SC_ERR_INVALID_ARGUMENT, "Buffer should not be empty");
        return -1;
    }

    if (len == 0) {
        return 0;
    }

    while (buffer->used + len + 2 > buffer->size)
    {
        buffer->size *= 2;
        char *tmp = SCRealloc(buffer->data, buffer->size * sizeof(char));
        if (tmp == NULL) {
            SCLogError(SC_ERR_MEM_ALLOC, "Error resizing JA3 buffer");
            return -1;
        }
        buffer->data = tmp;
    }

    return 0;
}

/**
 * \brief Append buffer to buffer.
 *
 * Append the second buffer to the first and then free it.
 *
 * \param buffer1 The first buffer.
 * \param buffer2 The second buffer.
 *
 * \retval 0 on success.
 * \retval -1 on failure.
 */
int Ja3BufferAppendBuffer(JA3Buffer *buffer1, JA3Buffer *buffer2)
{
    if (buffer1 == NULL || buffer2 == NULL) {
        SCLogError(SC_ERR_INVALID_ARGUMENT, "Buffers should not be NULL");
        return -1;
    }

    /* If buffer1 contains no data, then we just copy the second buffer
       instead of appending its data. */
    if (buffer1->data == NULL) {
        Ja3BufferFree(buffer1);
        *buffer1 = *buffer2;
        return 0;
    }

    int rc = Ja3BufferResizeIfFull(buffer1, buffer2->used);
    if (rc != 0) {
        Ja3BufferFree(buffer1);
        Ja3BufferFree(buffer2);
        return -1;
    }

    if (buffer2->used == 0) {
        buffer1->used += snprintf(buffer1->data + buffer1->used, buffer1->size -
                                  buffer1->used, ",");
    } else {
        buffer1->used += snprintf(buffer1->data + buffer1->used, buffer1->size -
                                  buffer1->used, ",%s", buffer2->data);
    }

    Ja3BufferFree(buffer2);

    return 0;
}

/**
 * \internal
 * \brief Return number of digits in number.
 *
 * \param num The number.
 *
 * \return digits Number of digits.
 */
static unsigned NumberOfDigits(unsigned num)
{
    if (num < 10) {
        return 1;
    }

    return 1 + NumberOfDigits(num / 10);
}

/**
 * \brief Add value to buffer.
 *
 * \param buffer The buffer.
 * \param value  The value.
 *
 * \retval 0 on success.
 * \retval -1 on failure.
 */
int Ja3BufferAddValue(JA3Buffer *buffer, int value)
{
    if (buffer == NULL) {
        SCLogError(SC_ERR_INVALID_ARGUMENT, "Buffer should not be NULL");
        return -1;
    }

    if (buffer->data == NULL) {
        buffer->data = SCMalloc(JA3_BUFFER_INITIAL_SIZE * sizeof(char));
        if (buffer->data == NULL) {
            SCLogError(SC_ERR_MEM_ALLOC,
                       "Error allocating memory for JA3 data");
            Ja3BufferFree(buffer);
            return -1;
        }
        buffer->size = JA3_BUFFER_INITIAL_SIZE;
    }

    unsigned value_len = NumberOfDigits(value);

    int rc = Ja3BufferResizeIfFull(buffer, value_len);
    if (rc != 0) {
        Ja3BufferFree(buffer);
        return -1;
    }

    if (buffer->used == 0) {
        buffer->used += snprintf(buffer->data, buffer->size, "%d", value);
    }
    else {
        buffer->used += snprintf(buffer->data + buffer->used, buffer->size -
                                 buffer->used, "-%d", value);
    }

    return 0;
}

/**
 * \brief Generate Ja3 hash string.
 *
 * \param buffer The Ja3 buffer.
 *
 * \retval pointer to hash string on success.
 * \retval NULL on failure.
 */
char *Ja3GenerateHash(JA3Buffer *buffer)
{

#ifdef HAVE_NSS
    if (buffer == NULL) {
        SCLogError(SC_ERR_INVALID_ARGUMENT, "Buffer should not be NULL");
        return NULL;
    }

    if (buffer->data == NULL) {
        SCLogError(SC_ERR_INVALID_VALUE, "Buffer data should not be NULL");
        return NULL;
    }

    char *ja3_hash = SCMalloc(MD5_STRING_LENGTH * sizeof(char));
    if (ja3_hash == NULL) {
        SCLogError(SC_ERR_MEM_ALLOC,
                   "Error allocating memory for JA3 hash");
        return NULL;
    }

    unsigned char md5[MD5_LENGTH];
    HASH_HashBuf(HASH_AlgMD5, md5, (unsigned char *)buffer->data, buffer->used);

    int i, x;
    for (i = 0, x = 0; x < MD5_LENGTH; x++) {
        i += snprintf(ja3_hash + i, MD5_STRING_LENGTH - i, "%02x", md5[x]);
    }

    return ja3_hash;
#else
    return NULL;
#endif /* HAVE_NSS */

}

