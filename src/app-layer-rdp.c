/* Copyright (C) 2019 Open Information Security Foundation
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
 * \author Zach Kelly <zach.kelly@lmco.com>
 *
 * Application layer parser for RDP
 */

#include "suricata-common.h"
#include "stream.h"
#include "conf.h"
#include "util-unittest.h"
#include "app-layer-detect-proto.h"
#include "app-layer-parser.h"
#include "app-layer-rdp.h"
#include "rust-rdp-rdp-gen.h"

void RegisterRdpParsers(void) {
    /* only register if enabled in config */
    if (ConfGetNode("app-layer.protocols.rdp") == NULL) {
        return;
    }
    SCLogDebug("Registering rdp parser");
    rs_rdp_register_parser();
}
