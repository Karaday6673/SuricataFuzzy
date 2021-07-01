************
Transactions
************

.. contents:: Table of Contents

_`General Concepts`
===================

Transactions are abstractions that help detecting and logging in Suricata. They also help during the detection phase,
when dealing with protocols that can have large PDUs, like TCP, in controlling state for partial rule matching, in case of rules that mention more than one field.

Transactions are implemented and stored in the per-flow state. The engine interacts with them using a set of callbacks the parser registers.

_`How the engine uses transactions`
===================================

Logging
~~~~~~~

Suricata controls when logging should happen based on transaction completeness. For simpler protocols, such as ``dns``
or ``ntp``, that will most
likely happen once per transaction, by the time of its completion. In other cases, like with HTTP, this may happen at intermediary states.

In ``OutputTxLog``, the engine will compare current state with the value defined for the logging to happen, per flow
direction (``logger->tc_log_progress``, ``logger->ts_log_progress``). If state is less than that value, the engine skips to
the next logger. Code snippet from: suricata/src/output-tx.c:

.. code-block:: c

    static TmEcode OutputTxLog(ThreadVars *tv, Packet *p, void *thread_data)
    {
        .
        .
        .
            if ((ts_eof && tc_eof) || last_pseudo) {
                SCLogDebug("EOF, so log now");
            } else {
                if (logger->LogCondition) {
                    int r = logger->LogCondition(tv, p, alstate, tx, tx_id);
                    if (r == FALSE) {
                        SCLogDebug("conditions not met, not logging");
                        goto next_logger;
                    }
                } else {
                    if (tx_progress_tc < logger->tc_log_progress) {
                        SCLogDebug("progress not far enough, not logging");
                        goto next_logger;
                    }

                    if (tx_progress_ts < logger->ts_log_progress) {
                        SCLogDebug("progress not far enough, not logging");
                        goto next_logger;
                    }
                }
             }
        .
        .
        .
    }

Rule Matching
~~~~~~~~~~~~~

Transaction progress is also used for certain keywords to know what is the minimum state before we can expect a match: until that, Suricata won't even try to look for the patterns.

As seen in ``DetectAppLayerMpmRegister2`` that has ``int progress`` as parameter, and ``DetectAppLayerInspectEngineRegister2``, which expects ``int tx_min_progress``, for instance. In the code snippet,
``HTTP2StateDataClient``, ``HTTP2StateDataServer`` and ``0`` are the values passed to the functions.


.. code-block:: c

    void DetectFiledataRegister(void)
    {
        .
        .
        DetectAppLayerMpmRegister2("file_data", SIG_FLAG_TOSERVER, 2,
                PrefilterMpmFiledataRegister, NULL,
                ALPROTO_HTTP2, HTTP2StateDataClient);
        DetectAppLayerMpmRegister2("file_data", SIG_FLAG_TOCLIENT, 2,
                PrefilterMpmFiledataRegister, NULL,
                ALPROTO_HTTP2, HTTP2StateDataServer);
        .
        .
        DetectAppLayerInspectEngineRegister2("file_data",
            ALPROTO_HTTP2, SIG_FLAG_TOCLIENT, HTTP2StateDataServer,
            DetectEngineInspectFiledata, NULL);
        DetectAppLayerInspectEngineRegister2(
                "file_data", ALPROTO_FTPDATA, SIG_FLAG_TOSERVER, 0, DetectEngineInspectFiledata, NULL);
        .
        .
    }

_`Progress Tracking`
====================

As a rule of thumb, transactions will follow a request-response model: if a transaction has had a request and a response, it is complete.

But if a protocol has situations where a request or response won’t expect or generate a message from its counterpart,
it is also possible to have uni-directional transactions. In such cases, transaction is set to complete at the moment of
creation.

For example, DNS responses may be considered as completed transactions, because they also contain the request data, so
all information needed for logging and detection can be found in the response.

In addition, for file transfer protocols, or similar ones where there may be several messages before the file exchange
is completed (NFS, SMB), it is possible to create a level of abstraction to handle such complexity. This could be achieved by adding phases to the protocol implemented model (e.g., protocol negotiation phase (SMB), request parsed (HTTP), and so on).

This is controlled by implementing states. In Suricata, those will be enums that are incremented as the parsing
progresses. A state will start at 0. The higher its value, the closer the transaction would be to completion.

The engine interacts with transactions state using a set of callbacks the parser registers. State is defined per flow direction (``STREAM_TOSERVER`` / ``STREAM_TOCLIENT``).

In Summary - Transactions and State
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

- Initial state value: ``0``
- Simpler scenarios: state is simply an int.  ``1`` represents transaction completion, per direction.
- Complex Transaction State in Suricata: ``enum`` (Rust: ``i32``). Completion is indicated by the highest enum value (some examples are: SSH, HTTP, DNS, SMB).

_`Examples`
===========

Enums
~~~~~

Code snippet from: rust/src/ssh/ssh.rs:

.. code-block:: rust

    pub enum SSHConnectionState {
        SshStateInProgress = 0,
        SshStateBannerWaitEol = 1,
        SshStateBannerDone = 2,
        SshStateFinished = 3,
    }

From src/app-layer-ftp.h:

.. code-block:: c

    enum {
        FTP_STATE_IN_PROGRESS,
        FTP_STATE_PORT_DONE,
        FTP_STATE_FINISHED,
    };


API Callbacks
~~~~~~~~~~~~~

In Rust, this is done via the RustParser struct. As seen in rust/src/applayer.rs:

.. code-block:: rust

    /// Rust parser declaration
    pub struct RustParser {
            .
            .
            .
        /// Progress values at which the tx is considered complete in a direction
        pub tx_comp_st_ts:      c_int,
        pub tx_comp_st_tc:      c_int,
        .
        .
        .
    }

In C, the callback API is:

.. code-block:: c

    void AppLayerParserRegisterStateProgressCompletionStatus(
        AppProto alproto, const int ts, const int tc)

Simple scenario described, in Rust:

rust/src/dhcp/dhcp.rs:

.. code-block:: rust

    tx_comp_st_ts: 1
    tx_comp_st_tc: 1

For SSH, this looks like this:

rust/src/ssh/ssh.rs:

.. code-block:: rust

    tx_comp_st_ts: SSHConnectionState::SshStateFinished as i32,
    tx_comp_st_tc: SSHConnectionState::SshStateFinished as i32,

In C, callback usage would be as follows:

src/app-layer-dcerpc.c:

.. code-block:: c

    AppLayerParserRegisterStateProgressCompletionStatus(ALPROTO_DCERPC, 1, 1);

src/app-layer-ftp.c:

.. code-block:: c

    AppLayerParserRegisterStateProgressCompletionStatus(
        ALPROTO_FTP, FTP_STATE_FINISHED, FTP_STATE_FINISHED);

Sequence Diagrams
~~~~~~~~~~~~~~~~~

A DNS transaction in Suricata can be considered unidirectional:

.. image:: img/DnsRequestUnidirectionalTransaction.png
  :width: 600
  :alt: A sequence diagram with two entities, Client and Server, with an arrow going from the Client to the Server, labeled "DNS Request". After that, there is a dotted line labeled "Transaction Completed".

An HTTP2 transaction is an example of a bidirectional transaction, in Suricata:

.. image:: img/HTTP2BidirectionalTransaction.png
  :width: 600
  :alt: A sequence diagram with two entities, Client and Server, with an arrow going from the Clientto the Server labeled "Request" and below that an arrow going from Server to Client labeled "Response". Below those arrows, a dotted line indicates that the transaction is completed.

A TLS Handshake is a more complex example, where several messages are exchanged before the transaction is considered completed:

.. image:: img/TlsHandshake.png
  :width: 600
  :alt: A sequence diagram with two entities, Client and Server, with an arrow going from the Client to the Server labeled "ClientHello" and below that an arrow going from Server to Client labeled "ServerHello". Below those arrows, several more follow from Server to Client and vice-versa, before a dotted line indicates that the transaction is finally completed.

_`Common words and abbreviations`
=================================

- al, applayer: application layer
- alproto: application layer protocol
- alstate: application layer state
- engine: refers to Suricata core detection logic
- flow: a bidirectional flow of packets with the same 5-tuple elements (protocol, source ip, destination ip, source port, destination port. Vlans can be added as well)
- PDU: Protocol Data Unit
- rs: rust
- tc: to client
- ts: to server
- tx: transaction
