/* Copyright (C) 2012 Open Information Security Foundation
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
 *  \file
 *
 *  \autor nPulse Technologies, LLC.
 *  \author Matt Keeler <mk@npulsetech.com>
 */

#ifndef __RUNMODE_NAPATECH_3GD_H__
#define __RUNMODE_NAPATECH_3GD_H__

#ifdef HAVE_NAPATECH_3GD
#include <nt.h>
#endif

int RunModeNapatech3GDAuto(DetectEngineCtx *);
int RunModeNapatech3GDAutoFp(DetectEngineCtx *);
int RunModeNapatech3GDWorkers(DetectEngineCtx *);
void RunModeNapatech3GDRegister(void);
const char *RunModeNapatech3GDGetDefaultMode(void);

#endif /* __RUNMODE_NAPATECH_3GD_H__ */
