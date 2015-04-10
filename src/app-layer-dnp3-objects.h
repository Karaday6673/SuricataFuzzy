/* Copyright (C) 2015 Open Information Security Foundation
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
 * \author Jason Ish <jason.ish@oisf.net>
 *
 * This file contains the types (definitions) of the DNP3 objects.
 */

#ifndef __APP_LAYER_DNP3_OBJECTS_H__
#define __APP_LAYER_DNP3_OBJECTS_H__

#define DNP3_OBJECT_CODE(group, variation) (group << 8 | variation)

/* START GENERATED CODE */
typedef struct DNP3ObjectG1V1_ {
    uint8_t state;
} DNP3ObjectG1V1;

typedef struct DNP3ObjectG1V2_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t chatter_filter:1;
    uint8_t reserved:1;
    uint8_t state:1;
} DNP3ObjectG1V2;

typedef struct DNP3ObjectG2V1_ {
    uint8_t state;
} DNP3ObjectG2V1;

typedef struct DNP3ObjectG2V2_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t chatter_filter:1;
    uint8_t reserved:1;
    uint8_t state:1;
    uint64_t timestamp;
} DNP3ObjectG2V2;

typedef struct DNP3ObjectG2V3_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t chatter_filter:1;
    uint8_t reserved:1;
    uint8_t state:1;
    uint16_t timestamp;
} DNP3ObjectG2V3;

typedef struct DNP3ObjectG3V1_ {
    uint8_t state;
} DNP3ObjectG3V1;

typedef struct DNP3ObjectG3V2_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t chatter_filter:1;
    uint8_t state:2;
} DNP3ObjectG3V2;

typedef struct DNP3ObjectG4V1_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t chatter_filter:1;
    uint8_t state:2;
} DNP3ObjectG4V1;

typedef struct DNP3ObjectG4V2_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t chatter_filter:1;
    uint8_t state:2;
    uint64_t timestamp;
} DNP3ObjectG4V2;

typedef struct DNP3ObjectG4V3_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t chatter_filter:1;
    uint8_t state:2;
    uint16_t relative_time_ms;
} DNP3ObjectG4V3;

typedef struct DNP3ObjectG10V1_ {
    uint8_t state;
} DNP3ObjectG10V1;

typedef struct DNP3ObjectG10V2_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t reserved0:1;
    uint8_t reserved1:1;
    uint8_t state:1;
} DNP3ObjectG10V2;

typedef struct DNP3ObjectG11V1_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t reserved0:1;
    uint8_t reserved1:1;
    uint8_t state:1;
} DNP3ObjectG11V1;

typedef struct DNP3ObjectG11V2_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t reserved0:1;
    uint8_t reserved1:1;
    uint8_t state:1;
    uint64_t timestamp;
} DNP3ObjectG11V2;

typedef struct DNP3ObjectG12V1_ {
    uint8_t opype:4;
    uint8_t qu:1;
    uint8_t cr:1;
    uint8_t tcc:2;
    uint8_t count;
    uint32_t ontime;
    uint32_t offtime;
    uint8_t status_code:7;
    uint8_t reserved:1;
} DNP3ObjectG12V1;

typedef struct DNP3ObjectG12V2_ {
    uint8_t opype:4;
    uint8_t qu:1;
    uint8_t cr:1;
    uint8_t tcc:2;
    uint8_t count;
    uint32_t ontime;
    uint32_t offtime;
    uint8_t status_code:7;
    uint8_t reserved:1;
} DNP3ObjectG12V2;

typedef struct DNP3ObjectG12V3_ {
    uint8_t point;
} DNP3ObjectG12V3;

typedef struct DNP3ObjectG13V1_ {
    uint8_t status_code:7;
    uint8_t commanded_state:1;
} DNP3ObjectG13V1;

typedef struct DNP3ObjectG13V2_ {
    uint8_t status_code:7;
    uint8_t commanded_state:1;
    uint64_t timestamp;
} DNP3ObjectG13V2;

typedef struct DNP3ObjectG20V1_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t rollover:1;
    uint8_t discontinuity:1;
    uint8_t reserved0:1;
    uint32_t count;
} DNP3ObjectG20V1;

typedef struct DNP3ObjectG20V2_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t rollover:1;
    uint8_t discontinuity:1;
    uint8_t reserved0:1;
    uint16_t count;
} DNP3ObjectG20V2;

typedef struct DNP3ObjectG20V3_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t rollover:1;
    uint8_t reserved0:1;
    uint8_t reserved1:1;
    uint32_t count;
} DNP3ObjectG20V3;

typedef struct DNP3ObjectG20V4_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t rollover:1;
    uint8_t reserved0:1;
    uint8_t reserved1:1;
    uint16_t count;
} DNP3ObjectG20V4;

typedef struct DNP3ObjectG20V5_ {
    uint32_t count;
} DNP3ObjectG20V5;

typedef struct DNP3ObjectG20V6_ {
    uint16_t count;
} DNP3ObjectG20V6;

typedef struct DNP3ObjectG20V7_ {
    uint32_t count;
} DNP3ObjectG20V7;

typedef struct DNP3ObjectG20V8_ {
    uint16_t count;
} DNP3ObjectG20V8;

typedef struct DNP3ObjectG21V1_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t rollover:1;
    uint8_t discontinuity:1;
    uint8_t reserved0:1;
    uint32_t count;
} DNP3ObjectG21V1;

typedef struct DNP3ObjectG21V2_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t rollover:1;
    uint8_t discontinuity:1;
    uint8_t reserved0:1;
    uint16_t count;
} DNP3ObjectG21V2;

typedef struct DNP3ObjectG21V3_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t rollover:1;
    uint8_t reserved0:1;
    uint8_t reserved1:1;
    uint32_t count;
} DNP3ObjectG21V3;

typedef struct DNP3ObjectG21V4_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t rollover:1;
    uint8_t reserved0:1;
    uint8_t reserved1:1;
    uint16_t count;
} DNP3ObjectG21V4;

typedef struct DNP3ObjectG21V5_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t rollover:1;
    uint8_t discontinuity:1;
    uint8_t reserved1:1;
    uint32_t count;
    uint64_t timestamp;
} DNP3ObjectG21V5;

typedef struct DNP3ObjectG21V6_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t rollover:1;
    uint8_t discontinuity:1;
    uint8_t reserved1:1;
    uint16_t count;
    uint64_t timestamp;
} DNP3ObjectG21V6;

typedef struct DNP3ObjectG21V7_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t rollover:1;
    uint8_t reserved0:1;
    uint8_t reserved1:1;
    uint32_t count;
    uint64_t timestamp;
} DNP3ObjectG21V7;

typedef struct DNP3ObjectG21V8_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t rollover:1;
    uint8_t reserved0:1;
    uint8_t reserved1:1;
    uint16_t count;
    uint64_t timestamp;
} DNP3ObjectG21V8;

typedef struct DNP3ObjectG21V9_ {
    uint32_t count;
} DNP3ObjectG21V9;

typedef struct DNP3ObjectG21V10_ {
    uint16_t count;
} DNP3ObjectG21V10;

typedef struct DNP3ObjectG21V11_ {
    uint32_t count;
} DNP3ObjectG21V11;

typedef struct DNP3ObjectG21V12_ {
    uint16_t count;
} DNP3ObjectG21V12;

typedef struct DNP3ObjectG22V1_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t rollover:1;
    uint8_t discontinuity:1;
    uint8_t reserved0:1;
    uint32_t count;
} DNP3ObjectG22V1;

typedef struct DNP3ObjectG22V2_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t rollover:1;
    uint8_t discontinuity:1;
    uint8_t reserved0:1;
    uint16_t count;
} DNP3ObjectG22V2;

typedef struct DNP3ObjectG22V3_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t rollover:1;
    uint8_t reserved0:1;
    uint8_t reserved1:1;
    uint32_t count;
} DNP3ObjectG22V3;

typedef struct DNP3ObjectG22V4_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t rollover:1;
    uint8_t reserved0:1;
    uint8_t reserved1:1;
    uint16_t count;
} DNP3ObjectG22V4;

typedef struct DNP3ObjectG22V5_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t rollover:1;
    uint8_t reserved0:1;
    uint8_t reserved1:1;
    uint32_t count;
    uint64_t timestamp;
} DNP3ObjectG22V5;

typedef struct DNP3ObjectG22V6_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t rollover:1;
    uint8_t discontinuity:1;
    uint8_t reserved0:1;
    uint16_t count;
    uint64_t timestamp;
} DNP3ObjectG22V6;

typedef struct DNP3ObjectG22V7_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t rollover:1;
    uint8_t reserved0:1;
    uint8_t reserved1:1;
    uint32_t count;
    uint64_t timestamp;
} DNP3ObjectG22V7;

typedef struct DNP3ObjectG22V8_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t rollover:1;
    uint8_t reserved0:1;
    uint8_t reserved1:1;
    uint16_t count;
    uint64_t timestamp;
} DNP3ObjectG22V8;

typedef struct DNP3ObjectG23V1_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t rollover:1;
    uint8_t discontinuity:1;
    uint8_t reserved0:1;
    uint32_t count;
} DNP3ObjectG23V1;

typedef struct DNP3ObjectG23V2_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t rollover:1;
    uint8_t reserved0:1;
    uint8_t reserved1:1;
    uint16_t count;
} DNP3ObjectG23V2;

typedef struct DNP3ObjectG23V3_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t rollover:1;
    uint8_t reserved0:1;
    uint8_t reserved1:1;
    uint32_t count;
} DNP3ObjectG23V3;

typedef struct DNP3ObjectG23V4_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t rollover:1;
    uint8_t reserved0:1;
    uint8_t reserved1:1;
    uint16_t count;
} DNP3ObjectG23V4;

typedef struct DNP3ObjectG23V5_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t rollover:1;
    uint8_t discontinuity:1;
    uint8_t reserved0:1;
    uint32_t count;
    uint64_t timestamp;
} DNP3ObjectG23V5;

typedef struct DNP3ObjectG23V6_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t rollover:1;
    uint8_t discontinuity:1;
    uint8_t reserved0:1;
    uint16_t count;
    uint64_t timestamp;
} DNP3ObjectG23V6;

typedef struct DNP3ObjectG23V7_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t rollover:1;
    uint8_t reserved0:1;
    uint8_t reserved1:1;
    uint32_t count;
    uint64_t timestamp;
} DNP3ObjectG23V7;

typedef struct DNP3ObjectG23V8_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t rollover:1;
    uint8_t reserved0:1;
    uint8_t reserved1:1;
    uint16_t count;
    uint64_t timestamp;
} DNP3ObjectG23V8;

typedef struct DNP3ObjectG30V1_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t over_range:1;
    uint8_t reference_err:1;
    uint8_t reserved0:1;
    int32_t value;
} DNP3ObjectG30V1;

typedef struct DNP3ObjectG30V2_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t over_range:1;
    uint8_t reference_err:1;
    uint8_t reserved0:1;
    int16_t value;
} DNP3ObjectG30V2;

typedef struct DNP3ObjectG30V3_ {
    int32_t value;
} DNP3ObjectG30V3;

typedef struct DNP3ObjectG30V4_ {
    int16_t value;
} DNP3ObjectG30V4;

typedef struct DNP3ObjectG30V5_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t over_range:1;
    uint8_t reference_err:1;
    uint8_t reserved0:1;
    float value;
} DNP3ObjectG30V5;

typedef struct DNP3ObjectG30V6_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t over_range:1;
    uint8_t reference_err:1;
    uint8_t reserved0:1;
    double value;
} DNP3ObjectG30V6;

typedef struct DNP3ObjectG31V1_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t over_range:1;
    uint8_t reference_err:1;
    uint8_t reserved0:1;
    int32_t value;
} DNP3ObjectG31V1;

typedef struct DNP3ObjectG31V2_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t over_range:1;
    uint8_t reference_err:1;
    uint8_t reserved0:1;
    int16_t value;
} DNP3ObjectG31V2;

typedef struct DNP3ObjectG31V3_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t over_range:1;
    uint8_t reference_err:1;
    uint8_t reserved0:1;
    int32_t value;
    uint64_t timestamp;
} DNP3ObjectG31V3;

typedef struct DNP3ObjectG31V4_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t over_range:1;
    uint8_t reference_err:1;
    uint8_t reserved0:1;
    int16_t value;
    uint64_t timestamp;
} DNP3ObjectG31V4;

typedef struct DNP3ObjectG31V5_ {
    int32_t value;
} DNP3ObjectG31V5;

typedef struct DNP3ObjectG31V6_ {
    int16_t value;
} DNP3ObjectG31V6;

typedef struct DNP3ObjectG31V7_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t over_range:1;
    uint8_t reference_err:1;
    uint8_t reserved0:1;
    float value;
} DNP3ObjectG31V7;

typedef struct DNP3ObjectG31V8_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t over_range:1;
    uint8_t reference_err:1;
    uint8_t reserved0:1;
    double value;
} DNP3ObjectG31V8;

typedef struct DNP3ObjectG32V1_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t over_range:1;
    uint8_t reference_err:1;
    uint8_t reserved0:1;
    int32_t value;
} DNP3ObjectG32V1;

typedef struct DNP3ObjectG32V2_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t over_range:1;
    uint8_t reference_err:1;
    uint8_t reserved0:1;
    int16_t value;
} DNP3ObjectG32V2;

typedef struct DNP3ObjectG32V3_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t over_range:1;
    uint8_t reference_err:1;
    uint8_t reserved0:1;
    int32_t value;
    uint64_t timestamp;
} DNP3ObjectG32V3;

typedef struct DNP3ObjectG32V4_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t over_range:1;
    uint8_t reference_err:1;
    uint8_t reserved0:1;
    int16_t value;
    uint64_t timestamp;
} DNP3ObjectG32V4;

typedef struct DNP3ObjectG32V5_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t over_range:1;
    uint8_t reference_err:1;
    uint8_t reserved0:1;
    float value;
} DNP3ObjectG32V5;

typedef struct DNP3ObjectG32V6_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t over_range:1;
    uint8_t reference_err:1;
    uint8_t reserved0:1;
    double value;
} DNP3ObjectG32V6;

typedef struct DNP3ObjectG32V7_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t over_range:1;
    uint8_t reference_err:1;
    uint8_t reserved0:1;
    float value;
    uint64_t timestamp;
} DNP3ObjectG32V7;

typedef struct DNP3ObjectG32V8_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t over_range:1;
    uint8_t reference_err:1;
    uint8_t reserved0:1;
    double value;
    uint64_t timestamp;
} DNP3ObjectG32V8;

typedef struct DNP3ObjectG33V1_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t over_range:1;
    uint8_t reference_err:1;
    uint8_t reserved0:1;
    int32_t value;
} DNP3ObjectG33V1;

typedef struct DNP3ObjectG33V2_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t over_range:1;
    uint8_t reference_err:1;
    uint8_t reserved0:1;
    int16_t value;
} DNP3ObjectG33V2;

typedef struct DNP3ObjectG33V3_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t over_range:1;
    uint8_t reference_err:1;
    uint8_t reserved0:1;
    int32_t value;
    uint64_t timestamp;
} DNP3ObjectG33V3;

typedef struct DNP3ObjectG33V4_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t over_range:1;
    uint8_t reference_err:1;
    uint8_t reserved0:1;
    int16_t value;
    uint64_t timestamp;
} DNP3ObjectG33V4;

typedef struct DNP3ObjectG33V5_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t over_range:1;
    uint8_t reference_err:1;
    uint8_t reserved0:1;
    float value;
} DNP3ObjectG33V5;

typedef struct DNP3ObjectG33V6_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t over_range:1;
    uint8_t reference_err:1;
    uint8_t reserved0:1;
    double value;
} DNP3ObjectG33V6;

typedef struct DNP3ObjectG33V7_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t over_range:1;
    uint8_t reference_err:1;
    uint8_t reserved0:1;
    float value;
    uint64_t timestamp;
} DNP3ObjectG33V7;

typedef struct DNP3ObjectG33V8_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t over_range:1;
    uint8_t reference_err:1;
    uint8_t reserved0:1;
    double value;
    uint64_t timestamp;
} DNP3ObjectG33V8;

typedef struct DNP3ObjectG34V1_ {
    uint16_t deadband_value;
} DNP3ObjectG34V1;

typedef struct DNP3ObjectG34V2_ {
    uint32_t deadband_value;
} DNP3ObjectG34V2;

typedef struct DNP3ObjectG34V3_ {
    float deadband_value;
} DNP3ObjectG34V3;

typedef struct DNP3ObjectG40V1_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t over_range:1;
    uint8_t reference_err:1;
    uint8_t reserved0:1;
    int32_t value;
} DNP3ObjectG40V1;

typedef struct DNP3ObjectG40V2_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t over_range:1;
    uint8_t reference_err:1;
    uint8_t reserved0:1;
    int16_t value;
} DNP3ObjectG40V2;

typedef struct DNP3ObjectG40V3_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t over_range:1;
    uint8_t reference_err:1;
    uint8_t reserved0:1;
    float value;
} DNP3ObjectG40V3;

typedef struct DNP3ObjectG40V4_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t over_range:1;
    uint8_t reference_err:1;
    uint8_t reserved0:1;
    double value;
} DNP3ObjectG40V4;

typedef struct DNP3ObjectG41V1_ {
    int32_t value;
    uint8_t control_status;
} DNP3ObjectG41V1;

typedef struct DNP3ObjectG41V2_ {
    int16_t value;
    uint8_t control_status;
} DNP3ObjectG41V2;

typedef struct DNP3ObjectG41V3_ {
    float value;
    uint8_t control_status;
} DNP3ObjectG41V3;

typedef struct DNP3ObjectG41V4_ {
    double value;
    uint8_t control_status;
} DNP3ObjectG41V4;

typedef struct DNP3ObjectG42V1_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t over_range:1;
    uint8_t reference_err:1;
    uint8_t reserved0:1;
    int32_t value;
} DNP3ObjectG42V1;

typedef struct DNP3ObjectG42V2_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t over_range:1;
    uint8_t reference_err:1;
    uint8_t reserved0:1;
    int16_t value;
} DNP3ObjectG42V2;

typedef struct DNP3ObjectG42V3_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t over_range:1;
    uint8_t reference_err:1;
    uint8_t reserved0:1;
    int32_t value;
    uint64_t timestamp;
} DNP3ObjectG42V3;

typedef struct DNP3ObjectG42V4_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t over_range:1;
    uint8_t reference_err:1;
    uint8_t reserved0:1;
    int16_t value;
    uint64_t timestamp;
} DNP3ObjectG42V4;

typedef struct DNP3ObjectG42V5_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t over_range:1;
    uint8_t reference_err:1;
    uint8_t reserved0:1;
    float value;
} DNP3ObjectG42V5;

typedef struct DNP3ObjectG42V6_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t over_range:1;
    uint8_t reference_err:1;
    uint8_t reserved0:1;
    double value;
} DNP3ObjectG42V6;

typedef struct DNP3ObjectG42V7_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t over_range:1;
    uint8_t reference_err:1;
    uint8_t reserved0:1;
    float value;
    uint64_t timestamp;
} DNP3ObjectG42V7;

typedef struct DNP3ObjectG42V8_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t over_range:1;
    uint8_t reference_err:1;
    uint8_t reserved0:1;
    double value;
    uint64_t timestamp;
} DNP3ObjectG42V8;

typedef struct DNP3ObjectG43V1_ {
    uint8_t status_code:7;
    uint8_t reserved0:1;
    int32_t commanded_value;
} DNP3ObjectG43V1;

typedef struct DNP3ObjectG43V2_ {
    uint8_t status_code:7;
    uint8_t reserved0:1;
    int16_t commanded_value;
} DNP3ObjectG43V2;

typedef struct DNP3ObjectG43V3_ {
    uint8_t status_code:7;
    uint8_t reserved0:1;
    int32_t commanded_value;
    uint64_t timestamp;
} DNP3ObjectG43V3;

typedef struct DNP3ObjectG43V4_ {
    uint8_t status_code:7;
    uint8_t reserved0:1;
    int16_t commanded_value;
    uint64_t timestamp;
} DNP3ObjectG43V4;

typedef struct DNP3ObjectG43V5_ {
    uint8_t status_code:7;
    uint8_t reserved0:1;
    float commanded_value;
} DNP3ObjectG43V5;

typedef struct DNP3ObjectG43V6_ {
    uint8_t status_code:7;
    uint8_t reserved0:1;
    double commanded_value;
} DNP3ObjectG43V6;

typedef struct DNP3ObjectG43V7_ {
    uint8_t status_code:7;
    uint8_t reserved0:1;
    float commanded_value;
    uint64_t timestamp;
} DNP3ObjectG43V7;

typedef struct DNP3ObjectG43V8_ {
    uint8_t status_code:7;
    uint8_t reserved0:1;
    double commanded_value;
    uint64_t timestamp;
} DNP3ObjectG43V8;

typedef struct DNP3ObjectG50V1_ {
    uint64_t timestamp;
} DNP3ObjectG50V1;

typedef struct DNP3ObjectG50V2_ {
    uint64_t timestamp;
    uint32_t interval;
} DNP3ObjectG50V2;

typedef struct DNP3ObjectG50V3_ {
    uint64_t timestamp;
} DNP3ObjectG50V3;

typedef struct DNP3ObjectG50V4_ {
    uint64_t timestamp;
    uint32_t interval_count;
    uint8_t interval_units;
} DNP3ObjectG50V4;

typedef struct DNP3ObjectG51V1_ {
    uint64_t timestamp;
} DNP3ObjectG51V1;

typedef struct DNP3ObjectG51V2_ {
    uint64_t timestamp;
} DNP3ObjectG51V2;

typedef struct DNP3ObjectG52V1_ {
    uint16_t delay_secs;
} DNP3ObjectG52V1;

typedef struct DNP3ObjectG52V2_ {
    uint16_t delay_ms;
} DNP3ObjectG52V2;

typedef struct DNP3ObjectG70V1_ {
    uint16_t filename_size;
    uint8_t filetype_code;
    uint8_t attribute_code;
    uint16_t start_record;
    uint16_t end_record;
    uint32_t file_size;
    uint64_t created_timestamp;
    uint16_t permission;
    uint32_t file_id;
    uint32_t owner_id;
    uint32_t group_id;
    uint8_t file_function_code;
    uint8_t status_code;
    char filename[65535];
    uint16_t data_size;
    char data[65535];
} DNP3ObjectG70V1;

typedef struct DNP3ObjectG70V2_ {
    uint16_t username_offset;
    uint16_t username_size;
    uint16_t password_offset;
    uint16_t password_size;
    uint32_t authentication_key;
    char username[65535];
    char password[65535];
} DNP3ObjectG70V2;

typedef struct DNP3ObjectG70V3_ {
    uint16_t filename_offset;
    uint16_t filename_size;
    uint64_t created;
    uint16_t permissions;
    uint32_t authentication_key;
    uint32_t file_size;
    uint16_t operational_mode;
    uint16_t maximum_block_size;
    uint16_t request_id;
    char filename[65535];
} DNP3ObjectG70V3;

typedef struct DNP3ObjectG70V4_ {
    uint32_t file_handle;
    uint32_t file_size;
    uint16_t maximum_block_size;
    uint16_t request_id;
    uint8_t status_code;
    char optional_text[255];
    uint8_t optional_text_len;
} DNP3ObjectG70V4;

typedef struct DNP3ObjectG70V5_ {
    uint32_t file_handle;
    uint32_t block_number;
    char file_data[255];
    uint8_t file_data_len;
} DNP3ObjectG70V5;

typedef struct DNP3ObjectG70V6_ {
    uint32_t file_handle;
    uint32_t block_number;
    uint8_t status_code;
    char optional_text[255];
    uint8_t optional_text_len;
} DNP3ObjectG70V6;

typedef struct DNP3ObjectG70V7_ {
    uint16_t filename_offset;
    uint16_t filename_size;
    uint16_t file_type;
    uint32_t file_size;
    uint64_t created_timestamp;
    uint16_t permissions;
    uint16_t request_id;
    char filename[65535];
} DNP3ObjectG70V7;

typedef struct DNP3ObjectG70V8_ {
    char file_specification[65535];
    uint16_t file_specification_len;
} DNP3ObjectG70V8;

typedef struct DNP3ObjectG80V1_ {
    uint8_t state;
} DNP3ObjectG80V1;

typedef struct DNP3ObjectG81V1_ {
    uint8_t fill_percentage:7;
    uint8_t overflow_state:1;
    uint8_t group;
    uint8_t variation;
} DNP3ObjectG81V1;

typedef struct DNP3ObjectG83V1_ {
    char vendor_code[5];
    uint16_t object_id;
    uint16_t length;
    uint8_t *data_objects;
} DNP3ObjectG83V1;

typedef struct DNP3ObjectG86V2_ {
    uint8_t rd:1;
    uint8_t wr:1;
    uint8_t st:1;
    uint8_t ev:1;
    uint8_t df:1;
    uint8_t padding0:1;
    uint8_t padding1:1;
    uint8_t padding2:1;
} DNP3ObjectG86V2;

typedef struct DNP3ObjectG102V1_ {
    uint8_t value;
} DNP3ObjectG102V1;

typedef struct DNP3ObjectG120V1_ {
    uint32_t csq;
    uint16_t usr;
    uint8_t mal;
    uint8_t reason;
    uint8_t *challenge_data;
    uint16_t challenge_data_len;
} DNP3ObjectG120V1;

typedef struct DNP3ObjectG120V2_ {
    uint32_t csq;
    uint16_t usr;
    uint8_t *mac_value;
    uint16_t mac_value_len;
} DNP3ObjectG120V2;

typedef struct DNP3ObjectG120V3_ {
    uint32_t csq;
    uint16_t user_number;
} DNP3ObjectG120V3;

typedef struct DNP3ObjectG120V4_ {
    uint16_t user_number;
} DNP3ObjectG120V4;

typedef struct DNP3ObjectG120V5_ {
    uint32_t ksq;
    uint16_t user_number;
    uint8_t key_wrap_alg;
    uint8_t key_status;
    uint8_t mal;
    uint16_t challenge_data_len;
    uint8_t *challenge_data;
    uint8_t *mac_value;
    uint16_t mac_value_len;
} DNP3ObjectG120V5;

typedef struct DNP3ObjectG120V6_ {
    uint32_t ksq;
    uint16_t usr;
    uint8_t *wrapped_key_data;
    uint16_t wrapped_key_data_len;
} DNP3ObjectG120V6;

typedef struct DNP3ObjectG120V7_ {
    uint32_t sequence_number;
    uint16_t usr;
    uint16_t association_id;
    uint8_t error_code;
    uint64_t time_of_error;
    char error_text[65535];
    uint16_t error_text_len;
} DNP3ObjectG120V7;

typedef struct DNP3ObjectG120V8_ {
    uint8_t key_change_method;
    uint8_t certificate_type;
    uint8_t *certificate;
    uint16_t certificate_len;
} DNP3ObjectG120V8;

typedef struct DNP3ObjectG120V9_ {
    uint8_t *mac_value;
    uint16_t mac_value_len;
} DNP3ObjectG120V9;

typedef struct DNP3ObjectG120V10_ {
    uint8_t key_change_method;
    uint8_t operation;
    uint32_t scs;
    uint16_t user_role;
    uint16_t user_role_expiry_interval;
    uint16_t username_len;
    uint16_t user_public_key_len;
    uint16_t certification_data_len;
    char username[65535];
    uint8_t *user_public_key;
    uint8_t *certification_data;
} DNP3ObjectG120V10;

typedef struct DNP3ObjectG120V11_ {
    uint8_t key_change_method;
    uint16_t username_len;
    uint16_t master_challenge_data_len;
    char username[65535];
    uint8_t *master_challenge_data;
} DNP3ObjectG120V11;

typedef struct DNP3ObjectG120V12_ {
    uint32_t ksq;
    uint16_t user_number;
    uint16_t challenge_data_len;
    uint8_t *challenge_data;
} DNP3ObjectG120V12;

typedef struct DNP3ObjectG120V13_ {
    uint32_t ksq;
    uint16_t user_number;
    uint16_t encrypted_update_key_len;
    uint8_t *encrypted_update_key_data;
} DNP3ObjectG120V13;

typedef struct DNP3ObjectG120V14_ {
    uint8_t *digital_signature;
    uint16_t digital_signature_len;
} DNP3ObjectG120V14;

typedef struct DNP3ObjectG120V15_ {
    uint8_t *mac;
    uint32_t mac_len;
} DNP3ObjectG120V15;

typedef struct DNP3ObjectG121V1_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t reserved0:1;
    uint8_t discontinuity:1;
    uint8_t reserved1:1;
    uint16_t association_id;
    uint32_t count_value;
} DNP3ObjectG121V1;

typedef struct DNP3ObjectG122V1_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t reserved0:1;
    uint8_t discontinuity:1;
    uint8_t reserved1:1;
    uint16_t association_id;
    uint32_t count_value;
} DNP3ObjectG122V1;

typedef struct DNP3ObjectG122V2_ {
    uint8_t online:1;
    uint8_t restart:1;
    uint8_t comm_lost:1;
    uint8_t remote_forced:1;
    uint8_t local_forced:1;
    uint8_t reserved0:1;
    uint8_t discontinuity:1;
    uint8_t reserved1:1;
    uint16_t association_id;
    uint32_t count_value;
    uint64_t timestamp;
} DNP3ObjectG122V2;

/* END GENERATED CODE */

int DNP3DecodeObject(int group, int variation, const uint8_t **buf,
    uint32_t *len, uint8_t prefix_code, uint32_t start, uint32_t count,
    DNP3PointList *);
DNP3PointList *DNP3PointListAlloc(void);
void DNP3FreeObjectPointList(int group, int variation, DNP3PointList *);

#endif /* __APP_LAYER_DNP3_OBJECTS_H__ */
