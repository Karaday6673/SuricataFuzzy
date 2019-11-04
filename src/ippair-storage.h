/* Copyright (C) 2007-2013 Open Information Security Foundation
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
 *
 * IPPair wrapper around storage api
 */

#ifndef __IPPAIR_STORAGE_H__
#define __IPPAIR_STORAGE_H__

#include "util/util-storage.h"
#include "ippair.h"

unsigned int IPPairStorageSize(void);

void *IPPairGetStorageById(IPPair *h, int id);
int IPPairSetStorageById(IPPair *h, int id, void *ptr);
void *IPPairAllocStorageById(IPPair *h, int id);

void IPPairFreeStorageById(IPPair *h, int id);
void IPPairFreeStorage(IPPair *h);

void RegisterIPPairStorageTests(void);

int IPPairStorageRegister(const char *name, const unsigned int size, void *(*Alloc)(unsigned int), void (*Free)(void *));

#endif /* __IPPAIR_STORAGE_H__ */
