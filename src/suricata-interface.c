/** \file
 *
 *  \author Angelo Mirabella <mirabellaa@vmware.com>
 *
 *  Interface to the suricata library.
 */

#include "suricata-interface.h"

#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>

#include "conf-struct-loader.h"
#include "counters.h"
#include "detect-engine.h"
#include "output-callback-stats.h"
#include "flow-manager.h"
#include "runmode-lib.h"
#include "source-lib.h"

#define SURICATA_PROGNAME "suricata"


/**
 * \brief Create a Suricata context.
 *
 * \param n_workers    Number of packet processing threads that the engine is expected to support.
 * \return SuricataCtx Pointer to the initialized Suricata context.
 */
SuricataCtx *suricata_create_ctx(int n_workers) {
    /* Create the SuricataCtx */
    if (n_workers == 0) {
        fprintf(stderr, "The number of suricata workers must be > 0");
        exit(EXIT_FAILURE);
    }

    SuricataCtx *ctx = calloc(1, sizeof(SuricataCtx));
    if (ctx == NULL) {
        fprintf(stderr, "SuricataCtx creation failed");
        exit(EXIT_FAILURE);
    }

    if (pthread_mutex_init(&ctx->lock, NULL) != 0) {
        fprintf(stderr, "SuricataCtx mutex creation failed");
        exit(EXIT_FAILURE);
    }

    ctx->n_workers = n_workers;

    /* Retrieve default configuration. */
    ctx->cfg = calloc(1, sizeof(SuricataCfg));
    if (ctx->cfg == NULL) {
        fprintf(stderr, "SuricataCfg creation failed");
        exit(EXIT_FAILURE);
    }
    *ctx->cfg = CfgGetDefault();

    /* Setup the inner suricata instance. */
    SuricataPreInit("suricata");

    return ctx;
}

/**
 * \brief Helper function to destroy a SuricataCtx.
 *
 * \param ctx            Pointer to SuricataCtx.
 */
static void suricata_destroy_ctx(SuricataCtx *ctx) {
    CfgFree(ctx->cfg);
    free(ctx->cfg);
    pthread_mutex_destroy(&ctx->lock);
    free(ctx);
}

/**
 * \brief Register a callback that is invoked for every alert.
 *
 * \param ctx            Pointer to SuricataCtx.
 * \param callback       Pointer to a callback function.
 */
void suricata_register_alert_cb(SuricataCtx *ctx, CallbackFuncAlert callback) {
    SCInstance *suri = GetInstance();
    suri->callbacks.alert = callback;

    /* Enable callback in the config. */
    CfgSet(ctx->cfg, "outputs.callback.alert.enabled", "yes");
}

/**
 * \brief Register a callback that is invoked for every fileinfo event.
 *
 * \param ctx            Pointer to SuricataCtx.
 * \param callback       Pointer to a callback function.
 */
void suricata_register_fileinfo_cb(SuricataCtx *ctx, CallbackFuncFileinfo callback) {
    SCInstance *suri = GetInstance();
    suri->callbacks.fileinfo = callback;

    /* Enable callback in the config. */
    CfgSet(ctx->cfg, "outputs.callback.fileinfo.enabled", "yes");
}
/**
 * \brief Register a callback that is invoked for every flow.
 *
 * \param ctx            Pointer to SuricataCtx.
 * \param callback       Pointer to a callback function.
 */
void suricata_register_flow_cb(SuricataCtx *ctx, CallbackFuncFlow callback) {
    SCInstance *suri = GetInstance();
    suri->callbacks.flow = callback;

    /* Enable callback in the config. */
    CfgSet(ctx->cfg, "outputs.callback.flow.enabled", "yes");
}

/**
 * \brief Register a callback that is invoked for every FlowSnip event.
 *
 * \param ctx            Pointer to SuricataCtx.
 * \param callback       Pointer to a callback function.
 */
void suricata_register_flowsnip_cb(SuricataCtx *ctx, CallbackFuncFlowSnip callback) {
    SCInstance *suri = GetInstance();
    suri->callbacks.flowsnip = callback;

    /* Enable callback in the config. */
    CfgSet(ctx->cfg, "outputs.callback.flow-snip.enabled", "yes");
}

/**
 * \brief Register a callback that is invoked for every HTTP event.
 *
 * \param ctx            Pointer to SuricataCtx.
 * \param callback       Pointer to a callback function.
 */
void suricata_register_http_cb(SuricataCtx *ctx, CallbackFuncHttp callback) {
    SCInstance *suri = GetInstance();
    suri->callbacks.http = callback;

    /* Enable callback in the config. */
    CfgSet(ctx->cfg, "outputs.callback.http.enabled", "yes");
}

/**
 * \brief Register a callback that is invoked for every NTA event.
 *
 * \param ctx            Pointer to SuricataCtx.
 * \param callback       Pointer to a callback function.
 */
void suricata_register_nta_cb(SuricataCtx *ctx, CallbackFuncNta callback) {
    SCInstance *suri = GetInstance();
    suri->callbacks.nta = callback;

    /* Enable callback in the config. */
    CfgSet(ctx->cfg, "outputs.callback.nta.enabled", "yes");
}

/**
 * \brief Register a callback that is invoked for every PreventAction event.
 *
 * \param ctx            Pointer to SuricataCtx.
 * \param callback       Pointer to a callback function.
 */
void suricata_register_prevent_action_cb(SuricataCtx *ctx, CallbackFuncPreventAction callback) {
    SCInstance *suri = GetInstance();
    suri->callbacks.prevent_action = callback;

    /* Enable callback in the config. */
    CfgSet(ctx->cfg, "outputs.callback.prevent-action.enabled", "yes");
}

/**
 * \brief Register a callback that is invoked for each signature that failed to load.
 *
 * \param ctx            Pointer to SuricataCtx.
 * \param user_ctx       Pointer to a user-defined context object.
 * \param callback       Pointer to a callback function.
 */
void suricata_register_sig_failed_loading_cb(SuricataCtx *ctx, void *user_ctx,
                                             CallbackFuncSigFailedLoading callback) {
    SCInstance *suri = GetInstance();
    suri->callbacks.sig_failed_loading.func = callback;
    suri->callbacks.sig_failed_loading.user_ctx = user_ctx;
}

/**
 * \brief Register a callback that is invoked before a candidate signature is inspected.
 *
 *        Such callback will be able to decide if a signature is relevant or modify its action via
 *        the return value:
 *         * -1: discard
 *         * 0: inspect signature without modifying its action
 *         * >0: inspect signature but modify its action first with the returned value
 *
 * \param ctx            Pointer to SuricataCtx.
 * \param callback       Pointer to a callback function.
 */
void suricata_register_sig_cb(SuricataCtx *ctx, CallbackFuncSigCandidate callback) {
    SCInstance *suri = GetInstance();
    suri->callbacks.sig_candidate = callback;
}

/**
 * \brief Register a callback that is invoked every time `suricata_get_stats` is invoked.
 *
 * \param ctx            Pointer to SuricataCtx.
 * \param user_ctx       Pointer to a user-defined context object.
 * \param callback       Pointer to a callback function.
 */
void suricata_register_stats_cb(SuricataCtx *ctx, void *user_ctx, CallbackFuncStats callback) {
    CallbackStatsRegisterCallback(user_ctx, callback);

    /* Enable stats globally and stats callback in the config. */
    CfgSet(ctx->cfg, "stats.enabled", "yes");
    CfgSet(ctx->cfg, "outputs.callback.stats.enabled", "yes");
}

/**
 * \brief Retrieve suricata stats.
 */
void suricata_get_stats(void) {
    StatsPoll();
}


/**
 * \brief Register a callback that is invoked for every log message.
 *
 * \param ctx            Pointer to SuricataCtx.
 * \param callback       Pointer to a callback function.
 */
void suricata_register_log_cb(SuricataCtx *ctx, CallbackFuncLog callback) {
    SCInstance *suri = GetInstance();
    suri->callbacks.log = callback;

    /* Enable callback in the config. Notice the logging id is hard-coded but it should be fine
     * since suricata right now has only 3 output modules for logging (console, file, syslog) */
    CfgSet(ctx->cfg, "logging.outputs.callback.enabled", "yes");
}

/**
 * \brief Set a configuration option.
 *
 * \param ctx            Pointer to SuricataCtx.
 * \param key            The configuration option key.
 * \param val            The configuration option value.
 *
 * \return               1 if set, 0 if not set.
 */
int suricata_config_set(SuricataCtx *ctx, const char *key, const char *val) {
    return CfgSet(ctx->cfg, key, val);
}

/**
 * \brief Load configuration from file.
 *
 * \param ctx            Pointer to SuricataCtx.
 * \param config_file    ilename of the yaml configuration to load.
 */
void suricata_config_load(SuricataCtx *ctx, const char *config_file) {
    if (config_file && CfgLoadYaml(config_file, ctx->cfg) != 0) {
        /* Loading the configuration from Yaml failed. */
        fprintf(stderr, "Failed loading config file: %s", config_file);
        exit(EXIT_FAILURE);
    }
}

/**
 * \brief Enable suricata IPS mode (testing only).
 */
void suricata_enable_ips_mode(void) {
    EngineModeSetIPS();
}

/**
 * \brief Initialize a Suricata context.
 *
 * \param ctx            Pointer to SuricataCtx.
 */
void suricata_init(SuricataCtx *ctx) {
    /* Set runmode and config in the suricata instance. */
    SCInstance *suri = GetInstance();
    suri->run_mode = RUNMODE_LIB;
    suri->set_logdir = true;
    suri->cfg = ctx->cfg;

    /* If we registered at least one callback, force enabling the callback output module. */
    int enabled = 0;
    if (suri->callbacks.alert != NULL || suri->callbacks.fileinfo != NULL ||
        suri->callbacks.flow != NULL || suri->callbacks.http != NULL ||
        suri->callbacks.nta != NULL) {
        enabled = 1;
    }

    if (enabled) {
        CfgSet(ctx->cfg, "outputs.callback.enabled", "yes");
    }

    /* Invoke engine initialization. */
    if (SuricataInit(SURICATA_PROGNAME) == EXIT_FAILURE) {
        GlobalsDestroy(GetInstance());
        suricata_destroy_ctx(ctx);
        exit(EXIT_FAILURE);
    }

    ctx->init_done = 1;
}

/**
 * \brief Initialize a Suricata worker.
 *
 * This function is meant to be invoked by a thread in charge of processing packets. The thread
 * is not managed by the library, i.e it needs to be created and destroyed by the user.
 * This function has to be invoked before "suricata_handle_packet" or "suricata_handle_stream".
 *
 * \param ctx       Pointer to the Suricata context.
 * \param interface The interface name this worker is linked to (optional).
 * \return          Pointer to the worker context.
 */
ThreadVars *suricata_initialise_worker_thread(SuricataCtx *ctx, const char *interface) {
    pthread_mutex_lock(&ctx->lock);

    if (ctx->n_workers_created == ctx->n_workers) {
        fprintf(stderr, "Maximum number of workers thread already allocated");
        return NULL;
    }

    ThreadVars *tv = RunModeCreateWorker(interface);
    ctx->n_workers_created++;
    pthread_mutex_unlock(&ctx->lock);

    return tv;
}

/**
 * \brief Register a per worker counter.
 *
 *
 * \param tv           Pointer to the per-thread structure.
 * \param counter_name The counter name.
 * \return id          Counter id for the newly registered counter, or the already present counter.
 */
uint16_t suricata_register_worker_counter(ThreadVars *tv, const char *counter_name) {
    return StatsRegisterCounter(counter_name, tv);
}

/**
 * \brief Register a per worker average counter.
 *
 * The registered counter holds the average of all the values assigned to it.
 *
 * \param tv           Pointer to the per-thread structure.
 * \param counter_name The counter name.
 * \return id          Counter id for the newly registered counter, or the already present counter.
 */
uint16_t suricata_register_worker_avg_counter(ThreadVars *tv, const char *counter_name) {
    return StatsRegisterAvgCounter(counter_name, tv);
}

/**
 * \brief Register a per worker max counter.
 *
 * The registered counter holds the maximum of all the values assigned to it.
 *
 * \param tv           Pointer to the per-thread structure.
 * \param counter_name The counter name.
 * \return id          Counter id for the newly registered counter, or the already present counter.
 */
uint16_t suricata_register_worker_max_counter(ThreadVars *tv, const char *counter_name) {
    return StatsRegisterMaxCounter(counter_name, tv);
}

/**
 * \brief Register a global counter.
 *
 * The registered counter is managed by the client application (not the library). Thread safety
 * needs to be taken care of if the counter is accessed by multiple threads.
 *
 * \param counter_name The counter name.
 * \param func         Function pointer used to retrieve the counter (uint64_t).
 */
void suricata_register_global_counter(const char *counter_name, uint64_t (*func)(void)) {
    StatsRegisterGlobalCounter(counter_name, func);
}

/**
 * \brief Complete initialization of a Suricata worker.
 *
 * This function is meant to be invoked after `suricata_initialise_worker_thread` and after
 * registering the per worker counters.
 *
 * \param tv           Pointer to the per-thread structure.
 */
void suricata_worker_post_init(ThreadVars *tv) {
    RunModeSpawnWorker(tv);
}

/**
 * \brief Adds a value to the worker counter.
 *
 *
 * \param tv           Pointer to the per-thread structure.
 * \param id           The counter id.
 * \param value        The value to add.
 */
void suricata_worker_counter_add(ThreadVars *tv, uint16_t id, uint64_t value) {
    StatsAddUI64(tv, id, value);
}

/**
 * \brief Increase the value of the worker counter.
 *
 *
 * \param tv           Pointer to the per-thread structure.
 * \param id           The counter id.
 */
void suricata_worker_counter_increase(ThreadVars *tv, uint16_t id) {
    StatsIncr(tv, id);
}

/**
 * \brief Set the value of the worker counter.
 *
 *
 * \param tv           Pointer to the per-thread structure.
 * \param id           The counter id.
 * \param value        The value to set.
 */
void suricata_worker_counter_set(ThreadVars *tv, uint16_t id, uint64_t value) {
    StatsSetUI64(tv, id, value);
}

/**
 * \brief Reset the value of the worker counter.
 *
 *
 * \param tv           Pointer to the per-thread structure.
 * \param id           The counter id.
 */
void suricata_worker_counter_reset(ThreadVars *tv, uint16_t id) {
    StatsReset(tv, id);
}

/**
 * \brief Suricata post initialization tasks.
 *
 * \param ctx Pointer to the Suricata context.
 */
void suricata_post_init(SuricataCtx *ctx) {
    /* Wait till all the workers have been created. */
    while (ctx->n_workers_created < ctx->n_workers) {
        usleep(100);
    }

    SuricataPostInit();
    ctx->post_init_done = 1;
}

/**
 * \brief Cleanup a Suricata worker.
 *
 * \param ctx Pointer to the Suricata context.
 * \param tv  Pointer to the worker context.
 */
void suricata_deinit_worker_thread(SuricataCtx *ctx, ThreadVars *tv) {
    pthread_mutex_lock(&ctx->lock);
    ctx->n_workers_done++;
    pthread_mutex_unlock(&ctx->lock);

    RunModeDestroyWorker(tv);
}


/**
 * \brief Feed a packet to the library.
 *
 * \param tv                    Pointer to the per-thread structure.
 * \param data                  Pointer to the raw packet.
 * \param datalink              Datalink type.
 * \param ts                    Timeval structure.
 * \param len                   Packet length.
 * \param ignore_pkt_checksum   Boolean indicating if we should ignore the packet checksum.
 * \param tenant_uuid           Tenant uuid (16 bytes) to associate a flow to a tenant.
 * \param tenant_id             Tenant id of the detection engine to use.
 * \param flags                 Packet flags (currently only used for rule profiling).
 * \param user_ctx              Pointer to a user-defined context object.
 * \return                      Error code.
 */
int suricata_handle_packet(ThreadVars *tv, const uint8_t *data, int datalink, struct timeval ts,
                           uint32_t len, int ignore_pkt_checksum, uint64_t *tenant_uuid,
                           uint32_t tenant_id, uint32_t flags, void *user_ctx) {
    return TmModuleLibHandlePacket(tv, data, datalink, ts, len, ignore_pkt_checksum, tenant_uuid,
                                   tenant_id, flags, user_ctx);
}

/** \brief Feed a single stream segment to the library.
 *
 * \param tv                    Pointer to the per-thread structure.
 * \param finfo                 Pointer to the flow information.
 * \param data                  Pointer to the raw packet.
 * \param len                   Packet length.
 * \param tenant_uuid           Tenant uuid (16 bytes) to associate a flow to a tenant.
 * \param tenant_id             Tenant id of the detection engine to use.
 * \param flags                 Packet flags (currently only used for rule profiling).
 * \param user_ctx              Pointer to a user-defined context object.
 * \return                      Error code.
 */
int suricata_handle_stream(ThreadVars *tv, FlowStreamInfo *finfo, const uint8_t *data,
                           uint32_t len, uint64_t *tenant_uuid, uint32_t tenant_id, uint32_t flags,
                           void *user_ctx) {
    return TmModuleLibHandleStream(tv, finfo, data, len, tenant_uuid, tenant_id, flags, user_ctx);
}

/**
 * \brief Reload the detection engine (rule set).
 *
 * \param ctx Pointer to the Suricata context.
 */
void suricata_engine_reload(SuricataCtx *ctx) {
    // Do nothing the engine is not yet fully initialized or a reload is already in progress.
    if (!ctx->post_init_done || DetectEngineReloadIsStart()) {
        return;
    }

    DetectEngineReloadStart();
    DetectEngineReload(GetInstance());
    DetectEngineReloadSetIdle();
}

/**
 * \brief Shutdown the library.
 *
 * \param ctx Pointer to the Suricata context.
 */
void suricata_shutdown(SuricataCtx *ctx) {
    /* Wait till all the workers are done */
    while(ctx->n_workers_done != ctx->n_workers_created) {
        usleep(10 * 1000);
    }

    if (ctx->post_init_done) {
        EngineDone(); /* needed only in offlne mode ?. */
        SuricataShutdown();
    }

    if (ctx->init_done) {
        GlobalsDestroy(GetInstance());
    }

    suricata_destroy_ctx(ctx);
}
