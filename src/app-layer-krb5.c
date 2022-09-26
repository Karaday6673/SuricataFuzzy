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
 * \author Pierre Chifflier <chifflier@wzdftpd.net>
 *
 * Parser for Kerberos v5 application layer running on UDP port 88.
 */

#include "suricata-common.h"

#include "app-layer-krb5.h"
#include "rust.h"

#ifdef UNITTESTS
#include "app-layer-parser.h"
#include "app-layer-detect-proto.h"
#include "util-unittest.h"
#include "conf.h"
#include "stream.h"
#endif
void RegisterKRB5Parsers(void)
{
    rs_register_krb5_parser();

#ifdef UNITTESTS
    AppLayerParserRegisterProtocolUnittests(IPPROTO_TCP, ALPROTO_KRB5,
        KRB5ParserRegisterTests);
#endif
}

#ifdef UNITTESTS
#endif

void KRB5ParserRegisterTests(void)
{
#ifdef UNITTESTS
#endif
}
