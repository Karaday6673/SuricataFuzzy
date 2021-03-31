/* Copyright (C) 2021 Open Information Security Foundation
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
 * \author Juliana Fajardini <jufajardini@oisf.net>
 *
 * Pgsql application layer detector and parser.
 *
 * This PostgreSQL offers basic initial support to the PostgreSQL on the wire
 * protocol (PGSQL Frontend/Backend protocol).
 */

#include "suricata-common.h"
#include "stream.h"
#include "conf.h"

#include "util-unittest.h"

#include "app-layer-detect-proto.h"
#include "app-layer-parser.h"

#include "app-layer-pgsql.h"
#include "rust.h"

void RegisterPgsqlParsers(void)
{
    SCLogDebug("Registering Rust pgsql parser.");
    rs_pgsql_register_parser();
#ifdef UNITTESTS
    AppLayerParserRegisterProtocolUnittests(IPPROTO_TCP, ALPROTO_PGSQL, PgsqlParserRegisterTests);
#endif
}

#ifdef UNITTESTS
#endif

void PgsqlParserRegisterTests(void)
{
#ifdef UNITTESTS
#endif
}
