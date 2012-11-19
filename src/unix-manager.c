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
 * \file
 *
 * \author Eric Leblond <eric@regit.org>
 */

#include "suricata-common.h"
#include "suricata.h"
#include "unix-manager.h"
#include "detect-engine.h"
#include "tm-threads.h"
#include "runmodes.h"
#include "conf.h"
#include "flow-manager.h"
#include "flow-timeout.h"
#include "stream-tcp.h"

#include "util-privs.h"
#include "util-debug.h"
#include "util-signal.h"
#include "output.h"
#include "host.h"
#include "defrag.h"

#include <sys/un.h>
#include <sys/stat.h>
#include <sys/types.h>

#ifdef BUILD_UNIX_SOCKET
#include <jansson.h>

#define SOCKET_PATH LOCAL_STATE_DIR "/run/suricata/"
#define SOCKET_FILENAME "suricata-command.socket"
#define SOCKET_TARGET SOCKET_PATH SOCKET_FILENAME

typedef struct PcapFiles_ {
    char *filename;
    char *output_dir;
    TAILQ_ENTRY(PcapFiles_) next;
} PcapFiles;

typedef struct UnixCommand_ {
    time_t start_timestamp;
    int socket;
    int client;
    struct sockaddr_un client_addr;
    int select_max;
    fd_set select_set;
    DetectEngineCtx *de_ctx;
    TAILQ_HEAD(, PcapFiles_) files;
    int running;
} UnixCommand;

static int unix_manager_file_task_running = 0;
static int unix_manager_file_task_failed = 0;

/**
 * \brief Create a command unix socket on system
 *
 * \retval 0 in case of error, 1 in case of success
 */
int UnixNew(UnixCommand * this)
{
    struct sockaddr_un addr;
    int len;
    int ret;
    int on = 1;
    char *sockettarget = NULL;
    char *socketname;

    this->start_timestamp = time(NULL);
    this->socket = -1;
    this->client = -1;
    this->select_max = 0;
    this->running = 0;

    TAILQ_INIT(&this->files);

    /* Create socket dir */
    ret = mkdir(SOCKET_PATH, S_IRWXU|S_IXGRP|S_IRGRP);
    if ( ret != 0 ) {
        int err = errno;
        if (err != EEXIST) {
            SCLogError(SC_ERR_OPENING_FILE,
                    "Cannot create socket directory %s: %s", SOCKET_PATH, strerror(err));
            return 0;
        }
    }

    if (ConfGet("unix-command.filename", &socketname) == 1) {
        int socketlen = strlen(SOCKET_PATH) + strlen(socketname) + 2;
        sockettarget = SCMalloc(socketlen);
        if (sockettarget == NULL) {
            SCLogError(SC_ERR_MEM_ALLOC, "Unable to allocate socket name");
            return 0;
        }
        snprintf(sockettarget, socketlen, "%s/%s", SOCKET_PATH, socketname);
        SCLogInfo("Use unix socket file '%s'.", sockettarget);
    }
    if (sockettarget == NULL) {
        sockettarget = SCStrdup(SOCKET_TARGET);
        if (sockettarget == NULL) {
            SCLogError(SC_ERR_MEM_ALLOC, "Unable to allocate socket name");
            return 0;
        }
    }

    /* Remove socket file */
    (void) unlink(sockettarget);

    /* set address */
    addr.sun_family = AF_UNIX;
    strlcpy(addr.sun_path, sockettarget, sizeof(addr.sun_path));
    addr.sun_path[sizeof(addr.sun_path) - 1] = 0;
    len = strlen(addr.sun_path) + sizeof(addr.sun_family);

    /* create socket */
    this->socket = socket(AF_UNIX, SOCK_STREAM, 0);
    if (this->socket == -1) {
        SCLogWarning(SC_ERR_OPENING_FILE,
                     "Unix Socket: unable to create UNIX socket %s: %s",
                     addr.sun_path, strerror(errno));
        return 0;
    }
    this->select_max = this->socket + 1;

    /* Set file mode: will not fully work on most system, the group
     * permission is not changed on some Linux and *BSD won't do the
     * chmod. */
    ret = fchmod(this->socket, S_IRUSR|S_IWUSR|S_IRGRP|S_IWGRP);
    if (ret == -1) {
        int err = errno;
        SCLogWarning(SC_ERR_INITIALIZATION,
                     "Unable to change permission on socket: %s (%d)",
                     strerror(err),
                     err);
    }
    /* set reuse option */
    ret = setsockopt(this->socket, SOL_SOCKET, SO_REUSEADDR,
                     (char *) &on, sizeof(on));
    if ( ret != 0 ) {
        SCLogWarning(SC_ERR_INITIALIZATION,
                     "Cannot set sockets options: %s.",  strerror(errno));
    }

    /* bind socket */
    ret = bind(this->socket, (struct sockaddr *) &addr, len);
    if (ret == -1) {
        SCLogWarning(SC_ERR_INITIALIZATION,
                     "Unix socket: UNIX socket bind(%s) error: %s",
                     sockettarget, strerror(errno));
        SCFree(sockettarget);
        return 0;
    }

    /* listen */
    if (listen(this->socket, 1) == -1) {
        SCLogWarning(SC_ERR_INITIALIZATION,
                     "Command server: UNIX socket listen() error: %s",
                     strerror(errno));
        SCFree(sockettarget);
        return 0;
    }
    SCFree(sockettarget);
    return 1;
}

/**
 * \brief Close the unix socket
 */
void UnixCommandClose(UnixCommand  *this)
{
    if (this->client == -1)
        return;
    SCLogInfo("Unix socket: close client connection");
    close(this->client);
    this->client = -1;
    this->select_max = this->socket + 1;
}

/**
 * \brief Callback function used to send message to socket
 */
int UnixCommandSendCallback(const char *buffer, size_t size, void *data)
{
    UnixCommand *this = (UnixCommand *) data;

    if (send(this->client, buffer, size, MSG_NOSIGNAL) == -1) {
        SCLogInfo("Unable to send block: %s", strerror(errno));
        return -1;
    }

    return 0;
}

#define UNIX_PROTO_VERSION_LENGTH 200
#define UNIX_PROTO_VERSION "0.1"

/**
 * \brief Accept a new client on unix socket
 *
 *  The function is called when a new user is detected
 *  in UnixMain(). It does the initial protocol negotiation
 *  with client.
 *
 * \retval 0 in case of error, 1 in case of success
 */
int UnixCommandAccept(UnixCommand *this)
{
    char buffer[UNIX_PROTO_VERSION_LENGTH + 1];
    json_t *client_msg;
    json_t *server_msg;
    json_t *version;
    json_error_t jerror;
    int ret;

    /* accept client socket */
    socklen_t len = sizeof(this->client_addr);
    this->client = accept(this->socket, (struct sockaddr *) &this->client_addr,
                          &len);
    if (this->client < 0) {
        SCLogInfo("Unix socket: accept() error: %s",
                  strerror(errno));
        return 0;
    }
    SCLogDebug("Unix socket: client connection");

    /* read client version */
    buffer[sizeof(buffer)-1] = 0;
    ret = recv(this->client, buffer, sizeof(buffer)-1, 0);
    if (ret < 0) {
        SCLogInfo("Command server: client doesn't send version");
        UnixCommandClose(this);
        return 0;
    }
    if (ret >= (int)(sizeof(buffer)-1)) {
        SCLogInfo("Command server: client message is too long, "
                  "disconnect him.");
        UnixCommandClose(this);
    }
    buffer[ret] = 0;

    client_msg = json_loads(buffer, 0, &jerror);
    if (client_msg == NULL) {
        SCLogInfo("Invalid command, error on line %d: %s\n", jerror.line, jerror.text);
        UnixCommandClose(this);
        return 0;
    }

    version = json_object_get(client_msg, "version");
    if(!json_is_string(version)) {
        SCLogInfo("error: version is not a string");
        UnixCommandClose(this);
        return 0;
    }

    /* check client version */
    if (strcmp(json_string_value(version), UNIX_PROTO_VERSION) != 0) {
        SCLogInfo("Unix socket: invalid client version: \"%s\"",
                json_string_value(version));
        UnixCommandClose(this);
        return 0;
    } else {
        SCLogInfo("Unix socket: client version: \"%s\"",
                json_string_value(version));
    }

    /* send answer */
    server_msg = json_object();
    if (server_msg == NULL) {
        UnixCommandClose(this);
        return 0;
    }
    json_object_set_new(server_msg, "return", json_string("OK"));

    if (json_dump_callback(server_msg, UnixCommandSendCallback, this, 0) == -1) {
        SCLogWarning(SC_ERR_SOCKET, "Unable to send command");
        UnixCommandClose(this);
        return 0;
    }

    /* client connected */
    SCLogInfo("Unix socket: client connected");
    if (this->socket < this->client)
        this->select_max = this->client + 1;
    else
        this->select_max = this->socket + 1;
    return 1;
}


static void PcapFilesFree(PcapFiles *cfile)
{
    if (cfile == NULL)
        return;
    if (cfile->filename)
        SCFree(cfile->filename);
    if (cfile->output_dir)
        SCFree(cfile->output_dir);
    SCFree(cfile);
}

/**
 * \brief Add file to file queue
 *
 * \param this a UnixCommand:: structure
 * \param filename absolute filename
 * \param output_dir absolute name of directory where log will be put
 *
 * \retval 0 in case of error, 1 in case of success
 */
int UnixListAddFile(UnixCommand *this, const char *filename, const char *output_dir)
{
    PcapFiles *cfile = NULL;
    if (filename == NULL || this == NULL)
        return 0;
    cfile = SCMalloc(sizeof(PcapFiles));
    if (cfile == NULL) {
        SCLogError(SC_ERR_MEM_ALLOC, "Unable to allocate new file");
        return 0;
    }
    memset(cfile, 0, sizeof(PcapFiles));

    cfile->filename = SCStrdup(filename);
    if (cfile->filename == NULL) {
        SCFree(cfile);
        SCLogError(SC_ERR_MEM_ALLOC, "Unable to dup filename");
        return 0;
    }

    if (output_dir) {
        cfile->output_dir = SCStrdup(output_dir);
        if (cfile->output_dir == NULL) {
            SCFree(cfile->filename);
            SCFree(cfile);
            SCLogError(SC_ERR_MEM_ALLOC, "Unable to dup output_dir");
            return 0;
        }
    }

    TAILQ_INSERT_TAIL(&this->files, cfile, next);
    return 1;
}

/**
 * \brief Handle the file queue
 *
 * This function check if there is currently a file
 * being parse. If it is not the case, it will start to
 * work on a new file. This implies to start a new 'pcap-file'
 * running mode after having set the file and the output dir.
 * This function also handles the cleaning of the previous
 * running mode.
 *
 * \param this a UnixCommand:: structure
 * \retval 0 in case of error, 1 in case of success
 */
int UnixPcapFilesHandle(UnixCommand *this)
{
    if (unix_manager_file_task_running == 1) {
        return 1;
    }
    if ((unix_manager_file_task_failed == 1) || (this->running == 1)) {
        if (unix_manager_file_task_failed) {
            SCLogInfo("Preceeding taks failed, cleaning the running mode");
        }
        unix_manager_file_task_failed = 0;
        this->running = 0;
        TmThreadKillThreadsFamily(TVT_MGMT);
        TmThreadClearThreadsFamily(TVT_MGMT);
        FlowForceReassembly();
        TmThreadKillThreadsFamily(TVT_PPT);
        TmThreadClearThreadsFamily(TVT_PPT);
        RunModeShutDown();
        SCPerfReleaseResources();
        /* thread killed, we can run non thread-safe shutdown functions */
        FlowShutdown();
        HostShutdown();
        StreamTcpFreeConfig(STREAM_VERBOSE);
        DefragDestroy();
        TmqResetQueues();
    }
    if (!TAILQ_EMPTY(&this->files)) {
        PcapFiles *cfile = TAILQ_FIRST(&this->files);
        TAILQ_REMOVE(&this->files, cfile, next);
        SCLogInfo("Starting run for '%s'", cfile->filename);
        unix_manager_file_task_running = 1;
        this->running = 1;
        if (ConfSet("pcap-file.file", cfile->filename, 1) != 1) {
            SCLogInfo("Can not set working file to '%s'", cfile->filename);
            PcapFilesFree(cfile);
            return 0;
        }
        if (cfile->output_dir) {
            if (ConfSet("default-log-dir", cfile->output_dir, 1) != 1) {
                SCLogInfo("Can not set output dir to '%s'", cfile->output_dir);
                PcapFilesFree(cfile);
                return 0;
            }
        }
        PcapFilesFree(cfile);
        SCPerfInitCounterApi();
        DefragInit();
        HostInitConfig(HOST_QUIET);
        FlowInitConfig(FLOW_QUIET);
        StreamTcpInitConfig(STREAM_VERBOSE);
        RunModeInitializeOutputs();
        /* FIXME add a variable to herit from the mode in the
         * configuration file */
        RunModeDispatch(RUNMODE_PCAP_FILE, NULL, this->de_ctx);
        FlowManagerThreadSpawn();
        SCPerfSpawnThreads();
        /* Un-pause all the paused threads */
        TmThreadContinueThreads();
    }
    return 1;
}

int UnixCommandBackgroundTasks(UnixCommand* this)
{
    int ret;

    ret = UnixPcapFilesHandle(this);
    if (ret == 0) {
        SCLogError(SC_ERR_OPENING_FILE, "Unable to handle PCAP file");
    }
    return 1;
}

/**
 * \brief return list of files in the queue
 *
 * \retval 0 in case of error, 1 in case of success
 */
static int UnixCommandFileList(UnixCommand* this, json_t *cmd, json_t* answer)
{
    int i = 0;
    PcapFiles *file;
    json_t *jdata;
    json_t *jarray;

    jdata = json_object();
    if (jdata == NULL) {
        json_object_set_new(answer, "message",
                            json_string("internal error at json object creation"));
        return 0;
    }
    jarray = json_array();
    if (jarray == NULL) {
        json_object_set_new(answer, "message",
                            json_string("internal error at json object creation"));
        return 0;
    }
    TAILQ_FOREACH(file, &this->files, next) {
        json_array_append(jarray, json_string(file->filename));
        /* FIXME need to decrement ? */
        i++;
    }
    json_object_set_new(jdata, "count", json_integer(i));
    json_object_set_new(jdata, "files", jarray);
    json_object_set_new(answer, "message", jdata);
    return 1;
}

static int UnixCommandFileNumber(UnixCommand* this, json_t *cmd, json_t* answer)
{
    int i = 0;
    PcapFiles *file;

    TAILQ_FOREACH(file, &this->files, next) {
        i++;
    }
    json_object_set_new(answer, "message", json_integer(i));
    return 1;
}

int UnixCommandFile(UnixCommand* this, json_t *cmd, json_t* answer)
{
    int ret;
    const char *filename;
    const char *output_dir;
#ifdef OS_WIN32
    struct _stat st;
#else
    struct stat st;
#endif /* OS_WIN32 */

    json_t *jarg = json_object_get(cmd, "filename");
    if(!json_is_string(jarg)) {
        SCLogInfo("error: command is not a string");
        return 0;
    }
    filename = json_string_value(jarg);
#ifdef OS_WIN32
    if(_stat(filename, &st) != 0) {
#else
    if(stat(filename, &st) != 0) {
#endif /* OS_WIN32 */
        json_object_set_new(answer, "message", json_string("File does not exist"));
        return 0;
    }

    json_t *oarg = json_object_get(cmd, "output-dir");
    if (oarg != NULL) {
        if(!json_is_string(oarg)) {
            SCLogInfo("error: output dir is not a string");
            return 0;
        }
        output_dir = json_string_value(oarg);
    }

#ifdef OS_WIN32
    if(_stat(output_dir, &st) != 0) {
#else
    if(stat(output_dir, &st) != 0) {
#endif /* OS_WIN32 */
        json_object_set_new(answer, "message", json_string("Output directory does not exist"));
        return 0;
    }

    ret = UnixListAddFile(this, filename, output_dir);
    switch(ret) {
        case 0:
            json_object_set_new(answer, "message", json_string("Unable to add file to list"));
            return 0;
        case 1:
            SCLogInfo("Added file '%s' to list", filename);
            json_object_set_new(answer, "message", json_string("Successfully added file to list"));
            return 1;
    }
    return 1;
}

/**
 * \brief Command dispatcher
 *
 * \param this a UnixCommand:: structure
 * \param command a string containing a json formatted
 * command
 *
 * \retval 0 in case of error, 1 in case of success
 */
int UnixCommandExecute(UnixCommand * this, char *command)
{
    int ret = 1;
    json_error_t error;
    json_t *jsoncmd = NULL;
    json_t *cmd = NULL;
    json_t *server_msg = json_object();
    const char * value;

    jsoncmd = json_loads(command, 0, &error);
    if (jsoncmd == NULL) {
        SCLogInfo("Invalid command, error on line %d: %s\n", error.line, error.text);
        return 0;
    }

    if (server_msg == NULL) {
        goto error;
    }

    cmd = json_object_get(jsoncmd, "command");
    if(!json_is_string(cmd)) {
        SCLogInfo("error: command is not a string");
        goto error_cmd;
    }
    value = json_string_value(cmd);

    if (!strcmp(value, "shutdown")) {
        json_object_set_new(server_msg, "message", json_string("Closing Suricata"));
        EngineStop();
    } else if (!strcmp(value, "reload-rules")) {
        if (suricata_ctl_flags != 0) {
            json_object_set_new(server_msg, "message",
                                json_string("Live rule swap no longer possible. Engine in shutdown mode."));
            ret = 0;
        } else {
            /* FIXME : need to check option value */
            UtilSignalHandlerSetup(SIGUSR2, SignalHandlerSigusr2Idle);
            DetectEngineSpawnLiveRuleSwapMgmtThread();
            json_object_set_new(server_msg, "message", json_string("Reloading rules"));
        }
    } else if (!strcmp(value, "pcap-file")) {
        cmd = json_object_get(jsoncmd, "arguments");
        if(!json_is_object(cmd)) {
            SCLogInfo("error: argument is not an object");
            goto error_cmd;
        }
        ret = UnixCommandFile(this, cmd, server_msg);
    } else if (!strcmp(value, "pcap-file-number")) {
        ret = UnixCommandFileNumber(this, cmd, server_msg);
    } else if (!strcmp(value, "pcap-file-list")) {
        ret = UnixCommandFileList(this, cmd, server_msg);
    } else {
        json_object_set_new(server_msg, "message", json_string("Unknown command"));
        ret = 0;
    }
    switch (ret) {
        case 0:
            json_object_set_new(server_msg, "return", json_string("NOK"));
            break;
        case 1:
            json_object_set_new(server_msg, "return", json_string("OK"));
            break;
    }

    /* send answer */
    if (json_dump_callback(server_msg, UnixCommandSendCallback, this, 0) == -1) {
        SCLogWarning(SC_ERR_SOCKET, "Unable to send command");
        goto error_cmd;
    }

    json_decref(jsoncmd);
    return ret;

error_cmd:
error:
    json_decref(jsoncmd);
    json_decref(server_msg);
    UnixCommandClose(this);
    return 0;
}

void UnixCommandRun(UnixCommand * this)
{
    char buffer[4096];
    int ret;
    ret = recv(this->client, buffer, sizeof(buffer) - 1, 0);
    if (ret <= 0) {
        if (ret == 0) {
            SCLogInfo("Unix socket: lost connection with client");
        } else {
            SCLogInfo("Unix socket: error on recv() from client: %s",
                      strerror(errno));
        }
        UnixCommandClose(this);
        return;
    }
    if (ret >= (int)(sizeof(buffer)-1)) {
        SCLogInfo("Command server: client command is too long, "
                  "disconnect him.");
        UnixCommandClose(this);
    }
    buffer[ret] = 0;
    UnixCommandExecute(this, buffer);
}

/**
 * \brief Select function
 *
 * \retval 0 in case of error, 1 in case of success
 */
int UnixMain(UnixCommand * this)
{
    struct timeval tv;
    int ret;

    /* Wait activity on the socket */
    FD_ZERO(&this->select_set);
    FD_SET(this->socket, &this->select_set);
    if (0 <= this->client)
        FD_SET(this->client, &this->select_set);
    tv.tv_sec = 0;
    tv.tv_usec = 200 * 1000;
    ret = select(this->select_max, &this->select_set, NULL, NULL, &tv);

    /* catch select() error */
    if (ret == -1) {
        /* Signal was catched: just ignore it */
        if (errno == EINTR) {
            return 1;
        }
        SCLogInfo("Command server: select() fatal error: %s", strerror(errno));
        return 0;
    }

    if (suricata_ctl_flags & (SURICATA_STOP | SURICATA_KILL)) {
        UnixCommandClose(this);
        return 1;
    }

    /* timeout: continue */
    if (ret == 0) {
        return 1;
    }

    if (0 <= this->client && FD_ISSET(this->client, &this->select_set)) {
        UnixCommandRun(this);
    }
    if (FD_ISSET(this->socket, &this->select_set)) {
        if (!UnixCommandAccept(this))
            return 0;
    }

    return 1;
}

/**
 * \brief Used to kill unix manager thread(s).
 *
 * \todo Kinda hackish since it uses the tv name to identify unix manager
 *       thread.  We need an all weather identification scheme.
 */
void UnixKillUnixManagerThread(void)
{
    ThreadVars *tv = NULL;
    int cnt = 0;

    SCCondSignal(&unix_manager_cond);

    SCMutexLock(&tv_root_lock);

    /* flow manager thread(s) is/are a part of mgmt threads */
    tv = tv_root[TVT_CMD];

    while (tv != NULL) {
        if (strcasecmp(tv->name, "UnixManagerThread") == 0) {
            TmThreadsSetFlag(tv, THV_KILL);
            TmThreadsSetFlag(tv, THV_DEINIT);

            /* be sure it has shut down */
            while (!TmThreadsCheckFlag(tv, THV_CLOSED)) {
                usleep(100);
            }
            cnt++;
        }
        tv = tv->next;
    }

    /* not possible, unless someone decides to rename UnixManagerThread */
    if (cnt == 0) {
        SCMutexUnlock(&tv_root_lock);
        abort();
    }

    SCMutexUnlock(&tv_root_lock);
    return;
}

void *UnixManagerThread(void *td)
{
    ThreadVars *th_v = (ThreadVars *)td;
    UnixCommand command;

    /* set the thread name */
    (void) SCSetThreadName(th_v->name);
    SCLogDebug("%s started...", th_v->name);

    command.de_ctx = (DetectEngineCtx *)th_v->tdata;

    th_v->sc_perf_pca = SCPerfGetAllCountersArray(&th_v->sc_perf_pctx);
    SCPerfAddToClubbedTMTable(th_v->name, &th_v->sc_perf_pctx);


    if (UnixNew(&command) == 0) {
        int failure_fatal = 0;
        SCLogError(SC_ERR_INITIALIZATION,
                     "Unable to create unix command socket");
        if (ConfGetBool("engine.init-failure-fatal", &failure_fatal) != 1) {
            SCLogDebug("ConfGetBool could not load the value.");
        }
        if (failure_fatal) {
            exit(EXIT_FAILURE);
        } else {
            TmThreadsSetFlag(th_v, THV_INIT_DONE|THV_RUNNING_DONE);
            pthread_exit((void *) 0);
        }
    }

    /* Set the threads capability */
    th_v->cap_flags = 0;
    SCDropCaps(th_v);

    /* Init Unix socket */


    TmThreadsSetFlag(th_v, THV_INIT_DONE);
    while (1) {
        UnixMain(&command);

        if (TmThreadsCheckFlag(th_v, THV_KILL)) {
            UnixCommandClose(&command);
            SCPerfSyncCounters(th_v, 0);
            break;
        }

        UnixCommandBackgroundTasks(&command);
    }
    TmThreadWaitForFlag(th_v, THV_DEINIT);

    TmThreadsSetFlag(th_v, THV_CLOSED);
    pthread_exit((void *) 0);
}


/** \brief spawn the unix socket manager thread */
void UnixManagerThreadSpawn(DetectEngineCtx *de_ctx, int mode)
{
    ThreadVars *tv_unixmgr = NULL;

    SCCondInit(&unix_manager_cond, NULL);

    tv_unixmgr = TmThreadCreateCmdThread("UnixManagerThread",
                                          UnixManagerThread, 0);

    if (tv_unixmgr == NULL) {
        SCLogError(SC_ERR_INITIALIZATION, "TmThreadsCreate failed");
        exit(EXIT_FAILURE);
    }
    if (TmThreadSpawn(tv_unixmgr) != TM_ECODE_OK) {
        SCLogError(SC_ERR_INITIALIZATION, "TmThreadSpawn failed");
        exit(EXIT_FAILURE);
    }
    if (mode == 1) {
        if (TmThreadsCheckFlag(tv_unixmgr, THV_RUNNING_DONE)) {
            SCLogError(SC_ERR_INITIALIZATION, "Unix socket init failed");
            exit(EXIT_FAILURE);
        }
    }
    return;
}

/**
 * \brief Used to kill unix manager thread(s).
 *
 * \todo Kinda hackish since it uses the tv name to identify flow manager
 *       thread.  We need an all weather identification scheme.
 */
void UnixSocketKillSocketThread(void)
{
    ThreadVars *tv = NULL;

    SCMutexLock(&tv_root_lock);

    /* flow manager thread(s) is/are a part of mgmt threads */
    tv = tv_root[TVT_MGMT];

    while (tv != NULL) {
        if (strcasecmp(tv->name, "UnixManagerThread") == 0) {
            /* If thread die during init it will have THV_RUNNING_DONE
             * set. So we can set the correct flag and exit.
             */
            if (TmThreadsCheckFlag(tv, THV_RUNNING_DONE)) {
                TmThreadsSetFlag(tv, THV_KILL);
                TmThreadsSetFlag(tv, THV_DEINIT);
                TmThreadsSetFlag(tv, THV_CLOSED);
                break;
            }
            TmThreadsSetFlag(tv, THV_KILL);
            TmThreadsSetFlag(tv, THV_DEINIT);
            /* be sure it has shut down */
            while (!TmThreadsCheckFlag(tv, THV_CLOSED)) {
                usleep(100);
            }
        }
        tv = tv->next;
    }

    SCMutexUnlock(&tv_root_lock);
    return;
}


void UnixSocketPcapFile(TmEcode tm)
{
    switch (tm) {
        case TM_ECODE_DONE:
            unix_manager_file_task_running = 0;
            break;
        case TM_ECODE_FAILED:
            unix_manager_file_task_running = 0;
            unix_manager_file_task_failed = 1;
            break;
        case TM_ECODE_OK:
            break;
    }
}

#else /* BUILD_UNIX_SOCKET */

void UnixManagerThreadSpawn(DetectEngineCtx *de_ctx, int mode)
{
    SCLogError(SC_ERR_UNIMPLEMENTED, "Unix socket is not compiled");
    return;
}

void UnixSocketKillSocketThread(void)
{
    return;
}

void UnixSocketPcapFile(TmEcode tm)
{
    return;
}

#endif /* BUILD_UNIX_SOCKET */
