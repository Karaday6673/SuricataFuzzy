/* Copyright (C) 2007-2012 Open Information Security Foundation
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
 * \author Roliers Jean-Paul <popof.fpn@gmail.co>
 *
 * Implements cryptographic functions.
 * Based on the libtomcrypt library ( http://libtom.org/?page=features&newsitems=5&whatfile=crypt )
 * 
 * Implementation of function using NSS is not linked with libtomcrypt.
 */

#include "suricata-common.h"
#include "suricata.h"
#include "util-crypt.h"
#ifdef HAVE_NSS
#include <sechash.h>
#endif

#ifndef HAVE_NSS

#define F0(x,y,z)  (z ^ (x & (y ^ z)))
#define F1(x,y,z)  (x ^ y ^ z)
#define F2(x,y,z)  ((x & y) | (z & (x | y)))
#define F3(x,y,z)  (x ^ y ^ z)


static int Sha1Compress(HashState *md, unsigned char *buf)
{
    uint32_t a,b,c,d,e,W[80],i;
    /* copy the state into 512-bits into W[0..15] */
    for (i = 0; i < 16; i++) {
        LOAD32H(W[i], buf + (4*i));
    }

    /* copy state */
    a = md->sha1.state[0];
    b = md->sha1.state[1];
    c = md->sha1.state[2];
    d = md->sha1.state[3];
    e = md->sha1.state[4];

    /* expand it */
    for (i = 16; i < 80; i++) {
        W[i] = ROL(W[i-3] ^ W[i-8] ^ W[i-14] ^ W[i-16], 1);
    }

    /* compress */
    /* round one */
    #define FF0(a,b,c,d,e,i) e = (ROLc(a, 5) + F0(b,c,d) + e + W[i] + 0x5a827999UL); b = ROLc(b, 30);
    #define FF1(a,b,c,d,e,i) e = (ROLc(a, 5) + F1(b,c,d) + e + W[i] + 0x6ed9eba1UL); b = ROLc(b, 30);
    #define FF2(a,b,c,d,e,i) e = (ROLc(a, 5) + F2(b,c,d) + e + W[i] + 0x8f1bbcdcUL); b = ROLc(b, 30);
    #define FF3(a,b,c,d,e,i) e = (ROLc(a, 5) + F3(b,c,d) + e + W[i] + 0xca62c1d6UL); b = ROLc(b, 30);

    for (i = 0; i < 20; ) {
       FF0(a,b,c,d,e,i++);
       FF0(e,a,b,c,d,i++);
       FF0(d,e,a,b,c,i++);
       FF0(c,d,e,a,b,i++);
       FF0(b,c,d,e,a,i++);
    }

    /* round two */
    for (; i < 40; )  {
       FF1(a,b,c,d,e,i++);
       FF1(e,a,b,c,d,i++);
       FF1(d,e,a,b,c,i++);
       FF1(c,d,e,a,b,i++);
       FF1(b,c,d,e,a,i++);
    }

    /* round three */
    for (; i < 60; )  {
       FF2(a,b,c,d,e,i++);
       FF2(e,a,b,c,d,i++);
       FF2(d,e,a,b,c,i++);
       FF2(c,d,e,a,b,i++);
       FF2(b,c,d,e,a,i++);
    }

    /* round four */
    for (; i < 80; )  {
       FF3(a,b,c,d,e,i++);
       FF3(e,a,b,c,d,i++);
       FF3(d,e,a,b,c,i++);
       FF3(c,d,e,a,b,i++);
       FF3(b,c,d,e,a,i++);
    }

    #undef FF0
    #undef FF1
    #undef FF2
    #undef FF3

    /* store */
    md->sha1.state[0] = md->sha1.state[0] + a;
    md->sha1.state[1] = md->sha1.state[1] + b;
    md->sha1.state[2] = md->sha1.state[2] + c;
    md->sha1.state[3] = md->sha1.state[3] + d;
    md->sha1.state[4] = md->sha1.state[4] + e;

    return SC_SHA_1_OK;
}

static int Sha1Init(HashState * md)
{
  if(md == NULL)
  {
      return SC_SHA_1_NOK;
  }
   md->sha1.state[0] = 0x67452301UL;
   md->sha1.state[1] = 0xefcdab89UL;
   md->sha1.state[2] = 0x98badcfeUL;
   md->sha1.state[3] = 0x10325476UL;
   md->sha1.state[4] = 0xc3d2e1f0UL;
   md->sha1.curlen = 0;
   md->sha1.length = 0;
   return SC_SHA_1_OK;
}

static int Sha1Process (HashState * md, const unsigned char *in, unsigned long inlen)
{
    if(md == NULL || in == NULL) {
        return SC_SHA_1_INVALID_ARG;
    }

    unsigned long n;
    int           err;

    if (md->sha1.curlen > sizeof(md->sha1.buf)) {
       return SC_SHA_1_INVALID_ARG;
    }
    while (inlen > 0) {
        if (md-> sha1.curlen == 0 && inlen >= 64) {
           if ((err = Sha1Compress(md, (unsigned char *)in)) != SC_SHA_1_OK) {
              return err;
           }
           md-> sha1 .length += 64 * 8;
           in             += 64;
           inlen          -= 64;
        } else {
           n = MIN(inlen, (64 - md-> sha1 .curlen));
           memcpy(md-> sha1 .buf + md-> sha1.curlen, in, (size_t)n);
           md-> sha1 .curlen += n;
           in             += n;
           inlen          -= n;
           if (md-> sha1 .curlen == 64) {
              if ((err = Sha1Compress(md, md-> sha1 .buf)) != SC_SHA_1_OK) {
                 return err;
              }
              md-> sha1 .length += 8*64;
              md-> sha1 .curlen = 0;
           }
       }
    }
    return SC_SHA_1_OK;
}



static int Sha1Done(HashState * md, unsigned char *out)
{
    int i;

    if (md  == NULL || out == NULL)
    {
        return SC_SHA_1_NOK;
    }

    if (md->sha1.curlen >= sizeof(md->sha1.buf)) {
       return SC_SHA_1_INVALID_ARG;
    }

    /* increase the length of the message */
    md->sha1.length += md->sha1.curlen * 8;

    /* append the '1' bit */
    md->sha1.buf[md->sha1.curlen++] = (unsigned char)0x80;

    /* if the length is currently above 56 bytes we append zeros
     * then compress.  Then we can fall back to padding zeros and length
     * encoding like normal.
     */
    if (md->sha1.curlen > 56) {
        while (md->sha1.curlen < 64) {
            md->sha1.buf[md->sha1.curlen++] = (unsigned char)0;
        }
        Sha1Compress(md, md->sha1.buf);
        md->sha1.curlen = 0;
    }

    /* pad upto 56 bytes of zeroes */
    while (md->sha1.curlen < 56) {
        md->sha1.buf[md->sha1.curlen++] = (unsigned char)0;
    }

    /* store length */
    STORE64H(md->sha1.length, md->sha1.buf+56);
    Sha1Compress(md, md->sha1.buf);

    /* copy output */
    for (i = 0; i < 5; i++) {
        STORE32H(md->sha1.state[i], out+(4*i));
    }

    memset(md, 0, sizeof(HashState));

    return SC_SHA_1_OK;
}

unsigned char* ComputeSHA1(unsigned char* buff, int bufflen)
{
    HashState md;
    unsigned char* lResult = (unsigned char*) SCMalloc((sizeof(unsigned char) * 20));
    if (lResult == NULL)
        return NULL;
    Sha1Init(&md);
    Sha1Process(&md, buff, bufflen);
    Sha1Done(&md, lResult);
    return lResult;
}

#else /* HAVE_NSS */

unsigned char* ComputeSHA1(unsigned char* buff, int bufflen)
{
    HASHContext *sha1_ctx = HASH_Create(HASH_AlgSHA1);
    unsigned char* lResult = NULL;
    unsigned int rlen;
    if (sha1_ctx == NULL) {
        return NULL;
    }

    lResult = (unsigned char*) SCMalloc((sizeof(unsigned char) * 20));
    if (lResult == NULL) {
        HASH_Destroy(sha1_ctx);
        return NULL;
    }
    HASH_Begin(sha1_ctx);
    HASH_Update(sha1_ctx, buff, bufflen);
    HASH_End(sha1_ctx, lResult, &rlen, (sizeof(unsigned char) * 20));
    HASH_Destroy(sha1_ctx);

    return lResult;
}

#endif /* HAVE_NSS */

static const char *b64codes = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

int Base64Encode(const unsigned char *in,  unsigned long inlen,
                        unsigned char *out, unsigned long *outlen)
{
   unsigned long i, len2, leven;
   unsigned char *p;
   if(in == NULL || out == NULL || outlen == NULL)
   {
       return SC_BASE64_INVALID_ARG;
   }
   /* valid output size ? */
   len2 = 4 * ((inlen + 2) / 3);
   if (*outlen < len2 + 1) {
      *outlen = len2 + 1;
      return SC_BASE64_OVERFLOW;
   }
   p = out;
   leven = 3*(inlen / 3);
   for (i = 0; i < leven; i += 3) {
       *p++ = b64codes[(in[0] >> 2) & 0x3F];
       *p++ = b64codes[(((in[0] & 3) << 4) + (in[1] >> 4)) & 0x3F];
       *p++ = b64codes[(((in[1] & 0xf) << 2) + (in[2] >> 6)) & 0x3F];
       *p++ = b64codes[in[2] & 0x3F];
       in += 3;
   }
   /* Pad it if necessary...  */
   if (i < inlen) {
       unsigned a = in[0];
       unsigned b = (i+1 < inlen) ? in[1] : 0;

       *p++ = b64codes[(a >> 2) & 0x3F];
       *p++ = b64codes[(((a & 3) << 4) + (b >> 4)) & 0x3F];
       *p++ = (i+1 < inlen) ? b64codes[(((b & 0xf) << 2)) & 0x3F] : '=';
       *p++ = '=';
   }
   /* append a NULL byte */
   *p = '\0';
   /* return ok */
   *outlen = p - out;
   return SC_BASE64_OK;
}

static const unsigned char map[256] = {
255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
255, 255, 255, 255, 255, 255, 255,  62, 255, 255, 255,  63,
 52,  53,  54,  55,  56,  57,  58,  59,  60,  61, 255, 255,
255, 254, 255, 255, 255,   0,   1,   2,   3,   4,   5,   6,
  7,   8,   9,  10,  11,  12,  13,  14,  15,  16,  17,  18,
 19,  20,  21,  22,  23,  24,  25, 255, 255, 255, 255, 255,
255,  26,  27,  28,  29,  30,  31,  32,  33,  34,  35,  36,
 37,  38,  39,  40,  41,  42,  43,  44,  45,  46,  47,  48,
 49,  50,  51, 255, 255, 255, 255, 255, 255, 255, 255, 255,
255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
255, 255, 255, 255 };

/**
   base64 decode a block of memory
   @param in       The base64 data to decode
   @param inlen    The length of the base64 data
   @param out      [out] The destination of the binary decoded data
   @param outlen   [in/out] The max size and resulting size of the decoded data
   @return CRYPT_OK if successful
*/
int Base64Decode(const unsigned char *in,  unsigned long inlen,
                        unsigned char *out, unsigned long *outlen)
{
   unsigned long t, x, y, z;
   unsigned char c;
   int           g;

   if (in == NULL)
       return SC_BASE64_INVALID_ARG;
   if (out == NULL)
       return SC_BASE64_INVALID_ARG;
   if (outlen == NULL)
       return SC_BASE64_INVALID_ARG;

   g = 3;
   for (x = y = z = t = 0; x < inlen; x++) {
       c = map[in[x]&0xFF];
       if (c == 255) continue;
       /* the final = symbols are read and used to trim the remaining bytes */
       if (c == 254) {
          c = 0;
          /* prevent g < 0 which would potentially allow an overflow later */
          if (--g < 0) {
             return SC_BASE64_INVALID_ARG;
          }
       } else if (g != 3) {
          /* we only allow = to be at the end */
          return SC_BASE64_INVALID_ARG;
       }

       t = (t<<6)|c;

       if (++y == 4) {
          if (z + g > *outlen) {
             return SC_BASE64_OVERFLOW;
          }
          out[z++] = (unsigned char)((t>>16)&255);
          if (g > 1) out[z++] = (unsigned char)((t>>8)&255);
          if (g > 2) out[z++] = (unsigned char)(t&255);
          y = t = 0;
       }
   }
   if (y != 0) {
       return SC_BASE64_INVALID_ARG;
   }
   *outlen = z;
   return SC_BASE64_OK;
}
