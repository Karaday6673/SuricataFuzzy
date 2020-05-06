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
 * \author Vadym Malakhatko <malakhatkovadim@gmail.com>
 *
 */

/**
 * \test Test matching on a simple client hello packet
 */
static int DetectSshHasshTest01(void) {
    Flow f;
    uint8_t sshbuf1[] = "SSH-2.0-MySSHClient-0.5.1\r\n";
    uint32_t sshlen1 = sizeof(sshbuf1) - 1;
    uint8_t sshbuf2[] = { 0x00, 0x00, 0x00, 0x03, 0x01, 21, 0x00};
    uint32_t sshlen2 = sizeof(sshbuf2);

    uint8_t sshbuf3[] = {0x00, 0x00, 0x05, 0x6c, 0x04, 0x14, 0x18, 0x70, 0xcb, 0xa4, 0xa3, 0xd4, 0xdc, 0x88, 0x6f, 0xfd,
                         0x76, 0x06, 0xcf, 0x36, 0x1b, 0xc6, 0x00, 0x00, 0x01, 0x0d, 0x63, 0x75, 0x72, 0x76, 0x65, 0x32,
                         0x35, 0x35, 0x31, 0x39, 0x2d, 0x73, 0x68, 0x61, 0x32, 0x35, 0x36, 0x2c, 0x63, 0x75, 0x72, 0x76,
                         0x65, 0x32, 0x35, 0x35, 0x31, 0x39, 0x2d, 0x73, 0x68, 0x61, 0x32, 0x35, 0x36, 0x40, 0x6c, 0x69,
                         0x62, 0x73, 0x73, 0x68, 0x2e, 0x6f, 0x72, 0x67, 0x2c, 0x65, 0x63, 0x64, 0x68, 0x2d, 0x73, 0x68,
                         0x61, 0x32, 0x2d, 0x6e, 0x69, 0x73, 0x74, 0x70, 0x32, 0x35, 0x36, 0x2c, 0x65, 0x63, 0x64, 0x68,
                         0x2d, 0x73, 0x68, 0x61, 0x32, 0x2d, 0x6e, 0x69, 0x73, 0x74, 0x70, 0x33, 0x38, 0x34, 0x2c, 0x65,
                         0x63, 0x64, 0x68, 0x2d, 0x73, 0x68, 0x61, 0x32, 0x2d, 0x6e, 0x69, 0x73, 0x74, 0x70, 0x35, 0x32,
                         0x31, 0x2c, 0x64, 0x69, 0x66, 0x66, 0x69, 0x65, 0x2d, 0x68, 0x65, 0x6c, 0x6c, 0x6d, 0x61, 0x6e,
                         0x2d, 0x67, 0x72, 0x6f, 0x75, 0x70, 0x2d, 0x65, 0x78, 0x63, 0x68, 0x61, 0x6e, 0x67, 0x65, 0x2d,
                         0x73, 0x68, 0x61, 0x32, 0x35, 0x36, 0x2c, 0x64, 0x69, 0x66, 0x66, 0x69, 0x65, 0x2d, 0x68, 0x65,
                         0x6c, 0x6c, 0x6d, 0x61, 0x6e, 0x2d, 0x67, 0x72, 0x6f, 0x75, 0x70, 0x31, 0x36, 0x2d, 0x73, 0x68,
                         0x61, 0x35, 0x31, 0x32, 0x2c, 0x64, 0x69, 0x66, 0x66, 0x69, 0x65, 0x2d, 0x68, 0x65, 0x6c, 0x6c,
                         0x6d, 0x61, 0x6e, 0x2d, 0x67, 0x72, 0x6f, 0x75, 0x70, 0x31, 0x38, 0x2d, 0x73, 0x68, 0x61, 0x35,
                         0x31, 0x32, 0x2c, 0x64, 0x69, 0x66, 0x66, 0x69, 0x65, 0x2d, 0x68, 0x65, 0x6c, 0x6c, 0x6d, 0x61,
                         0x6e, 0x2d, 0x67, 0x72, 0x6f, 0x75, 0x70, 0x31, 0x34, 0x2d, 0x73, 0x68, 0x61, 0x32, 0x35, 0x36,
                         0x2c, 0x64, 0x69, 0x66, 0x66, 0x69, 0x65, 0x2d, 0x68, 0x65, 0x6c, 0x6c, 0x6d, 0x61, 0x6e, 0x2d,
                         0x67, 0x72, 0x6f, 0x75, 0x70, 0x31, 0x34, 0x2d, 0x73, 0x68, 0x61, 0x31, 0x2c, 0x65, 0x78, 0x74,
                         0x2d, 0x69, 0x6e, 0x66, 0x6f, 0x2d, 0x63, 0x00, 0x00, 0x01, 0x66, 0x65, 0x63, 0x64, 0x73, 0x61,
                         0x2d, 0x73, 0x68, 0x61, 0x32, 0x2d, 0x6e, 0x69, 0x73, 0x74, 0x70, 0x32, 0x35, 0x36, 0x2d, 0x63,
                         0x65, 0x72, 0x74, 0x2d, 0x76, 0x30, 0x31, 0x40, 0x6f, 0x70, 0x65, 0x6e, 0x73, 0x73, 0x68, 0x2e,
                         0x63, 0x6f, 0x6d, 0x2c, 0x65, 0x63, 0x64, 0x73, 0x61, 0x2d, 0x73, 0x68, 0x61, 0x32, 0x2d, 0x6e,
                         0x69, 0x73, 0x74, 0x70, 0x33, 0x38, 0x34, 0x2d, 0x63, 0x65, 0x72, 0x74, 0x2d, 0x76, 0x30, 0x31,
                         0x40, 0x6f, 0x70, 0x65, 0x6e, 0x73, 0x73, 0x68, 0x2e, 0x63, 0x6f, 0x6d, 0x2c, 0x65, 0x63, 0x64,
                         0x73, 0x61, 0x2d, 0x73, 0x68, 0x61, 0x32, 0x2d, 0x6e, 0x69, 0x73, 0x74, 0x70, 0x35, 0x32, 0x31,
                         0x2d, 0x63, 0x65, 0x72, 0x74, 0x2d, 0x76, 0x30, 0x31, 0x40, 0x6f, 0x70, 0x65, 0x6e, 0x73, 0x73,
                         0x68, 0x2e, 0x63, 0x6f, 0x6d, 0x2c, 0x65, 0x63, 0x64, 0x73, 0x61, 0x2d, 0x73, 0x68, 0x61, 0x32,
                         0x2d, 0x6e, 0x69, 0x73, 0x74, 0x70, 0x32, 0x35, 0x36, 0x2c, 0x65, 0x63, 0x64, 0x73, 0x61, 0x2d,
                         0x73, 0x68, 0x61, 0x32, 0x2d, 0x6e, 0x69, 0x73, 0x74, 0x70, 0x33, 0x38, 0x34, 0x2c, 0x65, 0x63,
                         0x64, 0x73, 0x61, 0x2d, 0x73, 0x68, 0x61, 0x32, 0x2d, 0x6e, 0x69, 0x73, 0x74, 0x70, 0x35, 0x32,
                         0x31, 0x2c, 0x73, 0x73, 0x68, 0x2d, 0x65, 0x64, 0x32, 0x35, 0x35, 0x31, 0x39, 0x2d, 0x63, 0x65,
                         0x72, 0x74, 0x2d, 0x76, 0x30, 0x31, 0x40, 0x6f, 0x70, 0x65, 0x6e, 0x73, 0x73, 0x68, 0x2e, 0x63,
                         0x6f, 0x6d, 0x2c, 0x72, 0x73, 0x61, 0x2d, 0x73, 0x68, 0x61, 0x32, 0x2d, 0x35, 0x31, 0x32, 0x2d,
                         0x63, 0x65, 0x72, 0x74, 0x2d, 0x76, 0x30, 0x31, 0x40, 0x6f, 0x70, 0x65, 0x6e, 0x73, 0x73, 0x68,
                         0x2e, 0x63, 0x6f, 0x6d, 0x2c, 0x72, 0x73, 0x61, 0x2d, 0x73, 0x68, 0x61, 0x32, 0x2d, 0x32, 0x35,
                         0x36, 0x2d, 0x63, 0x65, 0x72, 0x74, 0x2d, 0x76, 0x30, 0x31, 0x40, 0x6f, 0x70, 0x65, 0x6e, 0x73,
                         0x73, 0x68, 0x2e, 0x63, 0x6f, 0x6d, 0x2c, 0x73, 0x73, 0x68, 0x2d, 0x72, 0x73, 0x61, 0x2d, 0x63,
                         0x65, 0x72, 0x74, 0x2d, 0x76, 0x30, 0x31, 0x40, 0x6f, 0x70, 0x65, 0x6e, 0x73, 0x73, 0x68, 0x2e,
                         0x63, 0x6f, 0x6d, 0x2c, 0x73, 0x73, 0x68, 0x2d, 0x65, 0x64, 0x32, 0x35, 0x35, 0x31, 0x39, 0x2c,
                         0x72, 0x73, 0x61, 0x2d, 0x73, 0x68, 0x61, 0x32, 0x2d, 0x35, 0x31, 0x32, 0x2c, 0x72, 0x73, 0x61,
                         0x2d, 0x73, 0x68, 0x61, 0x32, 0x2d, 0x32, 0x35, 0x36, 0x2c, 0x73, 0x73, 0x68, 0x2d, 0x72, 0x73,
                         0x61, 0x00, 0x00, 0x00, 0x6c, 0x63, 0x68, 0x61, 0x63, 0x68, 0x61, 0x32, 0x30, 0x2d, 0x70, 0x6f,
                         0x6c, 0x79, 0x31, 0x33, 0x30, 0x35, 0x40, 0x6f, 0x70, 0x65, 0x6e, 0x73, 0x73, 0x68, 0x2e, 0x63,
                         0x6f, 0x6d, 0x2c, 0x61, 0x65, 0x73, 0x31, 0x32, 0x38, 0x2d, 0x63, 0x74, 0x72, 0x2c, 0x61, 0x65,
                         0x73, 0x31, 0x39, 0x32, 0x2d, 0x63, 0x74, 0x72, 0x2c, 0x61, 0x65, 0x73, 0x32, 0x35, 0x36, 0x2d,
                         0x63, 0x74, 0x72, 0x2c, 0x61, 0x65, 0x73, 0x31, 0x32, 0x38, 0x2d, 0x67, 0x63, 0x6d, 0x40, 0x6f,
                         0x70, 0x65, 0x6e, 0x73, 0x73, 0x68, 0x2e, 0x63, 0x6f, 0x6d, 0x2c, 0x61, 0x65, 0x73, 0x32, 0x35,
                         0x36, 0x2d, 0x67, 0x63, 0x6d, 0x40, 0x6f, 0x70, 0x65, 0x6e, 0x73, 0x73, 0x68, 0x2e, 0x63, 0x6f,
                         0x6d, 0x00, 0x00, 0x00, 0x6c, 0x63, 0x68, 0x61, 0x63, 0x68, 0x61, 0x32, 0x30, 0x2d, 0x70, 0x6f,
                         0x6c, 0x79, 0x31, 0x33, 0x30, 0x35, 0x40, 0x6f, 0x70, 0x65, 0x6e, 0x73, 0x73, 0x68, 0x2e, 0x63,
                         0x6f, 0x6d, 0x2c, 0x61, 0x65, 0x73, 0x31, 0x32, 0x38, 0x2d, 0x63, 0x74, 0x72, 0x2c, 0x61, 0x65,
                         0x73, 0x31, 0x39, 0x32, 0x2d, 0x63, 0x74, 0x72, 0x2c, 0x61, 0x65, 0x73, 0x32, 0x35, 0x36, 0x2d,
                         0x63, 0x74, 0x72, 0x2c, 0x61, 0x65, 0x73, 0x31, 0x32, 0x38, 0x2d, 0x67, 0x63, 0x6d, 0x40, 0x6f,
                         0x70, 0x65, 0x6e, 0x73, 0x73, 0x68, 0x2e, 0x63, 0x6f, 0x6d, 0x2c, 0x61, 0x65, 0x73, 0x32, 0x35,
                         0x36, 0x2d, 0x67, 0x63, 0x6d, 0x40, 0x6f, 0x70, 0x65, 0x6e, 0x73, 0x73, 0x68, 0x2e, 0x63, 0x6f,
                         0x6d, 0x00, 0x00, 0x00, 0xd5, 0x75, 0x6d, 0x61, 0x63, 0x2d, 0x36, 0x34, 0x2d, 0x65, 0x74, 0x6d,
                         0x40, 0x6f, 0x70, 0x65, 0x6e, 0x73, 0x73, 0x68, 0x2e, 0x63, 0x6f, 0x6d, 0x2c, 0x75, 0x6d, 0x61,
                         0x63, 0x2d, 0x31, 0x32, 0x38, 0x2d, 0x65, 0x74, 0x6d, 0x40, 0x6f, 0x70, 0x65, 0x6e, 0x73, 0x73,
                         0x68, 0x2e, 0x63, 0x6f, 0x6d, 0x2c, 0x68, 0x6d, 0x61, 0x63, 0x2d, 0x73, 0x68, 0x61, 0x32, 0x2d,
                         0x32, 0x35, 0x36, 0x2d, 0x65, 0x74, 0x6d, 0x40, 0x6f, 0x70, 0x65, 0x6e, 0x73, 0x73, 0x68, 0x2e,
                         0x63, 0x6f, 0x6d, 0x2c, 0x68, 0x6d, 0x61, 0x63, 0x2d, 0x73, 0x68, 0x61, 0x32, 0x2d, 0x35, 0x31,
                         0x32, 0x2d, 0x65, 0x74, 0x6d, 0x40, 0x6f, 0x70, 0x65, 0x6e, 0x73, 0x73, 0x68, 0x2e, 0x63, 0x6f,
                         0x6d, 0x2c, 0x68, 0x6d, 0x61, 0x63, 0x2d, 0x73, 0x68, 0x61, 0x31, 0x2d, 0x65, 0x74, 0x6d, 0x40,
                         0x6f, 0x70, 0x65, 0x6e, 0x73, 0x73, 0x68, 0x2e, 0x63, 0x6f, 0x6d, 0x2c, 0x75, 0x6d, 0x61, 0x63,
                         0x2d, 0x36, 0x34, 0x40, 0x6f, 0x70, 0x65, 0x6e, 0x73, 0x73, 0x68, 0x2e, 0x63, 0x6f, 0x6d, 0x2c,
                         0x75, 0x6d, 0x61, 0x63, 0x2d, 0x31, 0x32, 0x38, 0x40, 0x6f, 0x70, 0x65, 0x6e, 0x73, 0x73, 0x68,
                         0x2e, 0x63, 0x6f, 0x6d, 0x2c, 0x68, 0x6d, 0x61, 0x63, 0x2d, 0x73, 0x68, 0x61, 0x32, 0x2d, 0x32,
                         0x35, 0x36, 0x2c, 0x68, 0x6d, 0x61, 0x63, 0x2d, 0x73, 0x68, 0x61, 0x32, 0x2d, 0x35, 0x31, 0x32,
                         0x2c, 0x68, 0x6d, 0x61, 0x63, 0x2d, 0x73, 0x68, 0x61, 0x31, 0x00, 0x00, 0x00, 0xd5, 0x75, 0x6d,
                         0x61, 0x63, 0x2d, 0x36, 0x34, 0x2d, 0x65, 0x74, 0x6d, 0x40, 0x6f, 0x70, 0x65, 0x6e, 0x73, 0x73,
                         0x68, 0x2e, 0x63, 0x6f, 0x6d, 0x2c, 0x75, 0x6d, 0x61, 0x63, 0x2d, 0x31, 0x32, 0x38, 0x2d, 0x65,
                         0x74, 0x6d, 0x40, 0x6f, 0x70, 0x65, 0x6e, 0x73, 0x73, 0x68, 0x2e, 0x63, 0x6f, 0x6d, 0x2c, 0x68,
                         0x6d, 0x61, 0x63, 0x2d, 0x73, 0x68, 0x61, 0x32, 0x2d, 0x32, 0x35, 0x36, 0x2d, 0x65, 0x74, 0x6d,
                         0x40, 0x6f, 0x70, 0x65, 0x6e, 0x73, 0x73, 0x68, 0x2e, 0x63, 0x6f, 0x6d, 0x2c, 0x68, 0x6d, 0x61,
                         0x63, 0x2d, 0x73, 0x68, 0x61, 0x32, 0x2d, 0x35, 0x31, 0x32, 0x2d, 0x65, 0x74, 0x6d, 0x40, 0x6f,
                         0x70, 0x65, 0x6e, 0x73, 0x73, 0x68, 0x2e, 0x63, 0x6f, 0x6d, 0x2c, 0x68, 0x6d, 0x61, 0x63, 0x2d,
                         0x73, 0x68, 0x61, 0x31, 0x2d, 0x65, 0x74, 0x6d, 0x40, 0x6f, 0x70, 0x65, 0x6e, 0x73, 0x73, 0x68,
                         0x2e, 0x63, 0x6f, 0x6d, 0x2c, 0x75, 0x6d, 0x61, 0x63, 0x2d, 0x36, 0x34, 0x40, 0x6f, 0x70, 0x65,
                         0x6e, 0x73, 0x73, 0x68, 0x2e, 0x63, 0x6f, 0x6d, 0x2c, 0x75, 0x6d, 0x61, 0x63, 0x2d, 0x31, 0x32,
                         0x38, 0x40, 0x6f, 0x70, 0x65, 0x6e, 0x73, 0x73, 0x68, 0x2e, 0x63, 0x6f, 0x6d, 0x2c, 0x68, 0x6d,
                         0x61, 0x63, 0x2d, 0x73, 0x68, 0x61, 0x32, 0x2d, 0x32, 0x35, 0x36, 0x2c, 0x68, 0x6d, 0x61, 0x63,
                         0x2d, 0x73, 0x68, 0x61, 0x32, 0x2d, 0x35, 0x31, 0x32, 0x2c, 0x68, 0x6d, 0x61, 0x63, 0x2d, 0x73,
                         0x68, 0x61, 0x31, 0x00, 0x00, 0x00, 0x1a, 0x6e, 0x6f, 0x6e, 0x65, 0x2c, 0x7a, 0x6c, 0x69, 0x62,
                         0x40, 0x6f, 0x70, 0x65, 0x6e, 0x73, 0x73, 0x68, 0x2e, 0x63, 0x6f, 0x6d, 0x2c, 0x7a, 0x6c, 0x69,
                         0x62, 0x00, 0x00, 0x00, 0x1a, 0x6e, 0x6f, 0x6e, 0x65, 0x2c, 0x7a, 0x6c, 0x69, 0x62, 0x40, 0x6f,
                         0x70, 0x65, 0x6e, 0x73, 0x73, 0x68, 0x2e, 0x63, 0x6f, 0x6d, 0x2c, 0x7a, 0x6c, 0x69, 0x62, 0x00,
                         0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00};
    uint32_t sshlen3 = sizeof(sshbuf3);

    TcpSession ssn;
    Packet *p = NULL;
    Signature *s = NULL;
    ThreadVars th_v;
    DetectEngineThreadCtx *det_ctx = NULL;
    AppLayerParserThreadCtx *alp_tctx = AppLayerParserThreadCtxAlloc();

    memset(&th_v, 0, sizeof(th_v));
    memset(&f, 0, sizeof(f));
    memset(&ssn, 0, sizeof(ssn));

    p = UTHBuildPacketReal(sshbuf3, sshlen3, IPPROTO_TCP,
                                   "192.168.1.5", "192.168.1.1",
                                   59070, 22);

    FLOW_INITIALIZE(&f);
    f.protoctx = (void *)&ssn;
    p->flow = &f;
    p->flowflags |= FLOW_PKT_TOSERVER;
    p->flowflags |= FLOW_PKT_ESTABLISHED;
    p->flags |= PKT_HAS_FLOW|PKT_STREAM_EST;
    f.alproto = ALPROTO_SSH;
    f.proto = IPPROTO_TCP;

    StreamTcpInitConfig(TRUE);

    DetectEngineCtx *de_ctx = DetectEngineCtxInit();
    FAIL_IF (de_ctx == NULL);

    de_ctx->flags |= DE_QUIET;

    s = de_ctx->sig_list = SigInit(de_ctx, "alert ssh any any -> any any ("
                                           "msg:\"match SSH hash\"; " 
                                           "hassh; content:\"ec7378c1a92f5a8dde7e8b7a1ddf33d1\"; "
                                           "sid:1;)");
    FAIL_IF(s == NULL);

    SigGroupBuild(de_ctx);
    DetectEngineThreadCtxInit(&th_v, (void *)de_ctx, (void *)&det_ctx);

    FLOWLOCK_WRLOCK(&f);

    int r = AppLayerParserParse(NULL, alp_tctx, &f, ALPROTO_SSH,
                                STREAM_TOSERVER, sshbuf1, sshlen1);
    FAIL_IF_NOT(r == 0);
    r = AppLayerParserParse(NULL, alp_tctx, &f, ALPROTO_SSH, STREAM_TOSERVER,
                            sshbuf2, sshlen2);
    FAIL_IF_NOT (r == 0);

    r = AppLayerParserParse(NULL, alp_tctx, &f, ALPROTO_SSH,
                                STREAM_TOSERVER, sshbuf3, sshlen3);
    FAIL_IF_NOT(r == 0); 

    FLOWLOCK_UNLOCK(&f);

    void *ssh_state = f.alstate;
    FAIL_IF (ssh_state == NULL);
    /* do detect */
    SigMatchSignatures(&th_v, de_ctx, det_ctx, p);

    FAIL_IF_NOT(PacketAlertCheck(p, 1));
    
    SigGroupCleanup(de_ctx);
    SigCleanSignatures(de_ctx);

    DetectEngineThreadCtxDeinit(&th_v, (void *)det_ctx);
    DetectEngineCtxFree(de_ctx);

    StreamTcpFreeConfig(TRUE);
    FLOW_DESTROY(&f);

    UTHFreePackets(&p, 1);

    if (alp_tctx != NULL)
        AppLayerParserThreadCtxFree(alp_tctx);
    PASS;
}

static void DetectSshHasshRegisterTests(void)
{
    UtRegisterTest("DetectSshHasshTest01", DetectSshHasshTest01);
}
