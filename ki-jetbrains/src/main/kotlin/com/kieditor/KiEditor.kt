package com.kieditor

import com.intellij.openapi.Disposable
import com.intellij.openapi.components.Service
import com.intellij.openapi.components.serviceAsync
import com.intellij.openapi.diagnostic.thisLogger
import com.intellij.openapi.project.Project
import com.kieditor.exceptions.MessageNeverUsedException
import com.kieditor.exceptions.MessageRequestOnlyException
import com.kieditor.exceptions.OutputMessageException
import com.kieditor.protocol.InputMessage
import com.kieditor.protocol.InputMessageWrapper
import com.kieditor.protocol.OutputMessage
import com.kieditor.protocol.OutputMessageWrapper
import io.ktor.client.*
import io.ktor.client.engine.cio.*
import io.ktor.client.plugins.websocket.*
import io.ktor.websocket.*
import kotlinx.coroutines.*
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json
import java.io.File
import java.util.concurrent.ConcurrentHashMap
import kotlin.coroutines.Continuation
import kotlin.coroutines.resume
import kotlin.coroutines.suspendCoroutine
import kotlin.time.Duration.Companion.minutes
import kotlin.time.Duration.Companion.seconds

@Service(Service.Level.PROJECT)
class KiEditor(val project: Project, val scope: CoroutineScope) : Disposable {

    private val logger = thisLogger()

    private val client = HttpClient(CIO) {
        install(WebSockets.Plugin) {
            pingInterval = 1.minutes
        }
    }

    private val webSocketSession: WebSocketSession

    private var nextMessageId: UInt = 1u // note: 0 is a special id used for events
    private val pendingRequests = ConcurrentHashMap<UInt, Continuation<OutputMessageWrapper>>()

    init {
        val port = startKi()

        webSocketSession = runBlocking {
            client.webSocketSession(host = "localhost", port = port)
        }

        scope.launch {
            eventLoop()
        }
    }

    private fun startKi(): Int {
        // todo
        val process = ProcessBuilder("./target/debug/ki", "@", "embed", ".")
            .directory(File("/home/exidex/IdeaProjects/ki-editor"))
            .redirectErrorStream(true)
            .start()

        val deferredPort = CompletableDeferred<Int>()
        val portRegex = "^KI_LISTENING_ON=(.+)$".toRegex()

        scope.launch {
            val reader = process.inputReader()

            while (true) {
                val line = withContext(Dispatchers.IO) { reader.readLine() }

                logger.info("Ki Editor: $line")

                if (!deferredPort.isCompleted && line.matches(portRegex)) {
                    val (port) = portRegex.matchEntire(line)!!.destructured
                    deferredPort.complete(port.toInt())
                }
            }
        }

        // todo can we remove this blocking call?
        val port = runBlocking { deferredPort.await() }

        logger.info("Ki Editor started at port: $port")

        return port
    }

    override fun dispose() {
        runBlocking {
            webSocketSession.close()
        }

        client.close()
    }

    private suspend fun eventLoop() {
        while (true) {
            when (val message = webSocketSession.incoming.receive()) {
                is Frame.Ping -> {}
                is Frame.Pong -> {}

                is Frame.Text -> {
                    val messageString = message.readText()
                    val messageWrapper = KiJson.decodeFromString<OutputMessageWrapper>(messageString)

                    logger.info("Received text from ws connection: $messageWrapper")

                    handleOutputMessage(messageWrapper)
                }

                is Frame.Close -> {
                    logger.error("Websocket session was closed, reason: ${message.readReason()}")
                    break
                }

                is Frame.Binary -> throw RuntimeException("Received unexpected binary message")
            }
        }

    }

    private suspend fun handleOutputMessage(wrapper: OutputMessageWrapper) {
        if (wrapper.id == 0u) {
            handleNotification(wrapper.message)
            return
        }

        val continuation = pendingRequests.remove(wrapper.id)
            ?: throw RuntimeException("There's no pending request for ${wrapper.id}") // todo should this stop the event loop?

        continuation.resume(wrapper)
    }

    private suspend fun handleNotification(message: OutputMessage) {
        when (message) {
            // system
            is OutputMessage.Ping -> throw MessageRequestOnlyException()
            is OutputMessage.Error -> throw MessageRequestOnlyException()

            // buffer
            is OutputMessage.BufferDiff -> project.serviceAsync<KiBufferManager>().handleBufferDiff(message)
            is OutputMessage.BufferOpen -> throw MessageNeverUsedException()
            is OutputMessage.BufferSave -> project.serviceAsync<KiBufferManager>().handleBufferSave(message)

            // selection
            is OutputMessage.SelectionUpdate -> TODO()

            // mode
            is OutputMessage.ModeChange -> TODO()
            is OutputMessage.SelectionModeChange -> TODO()

            // viewport
            is OutputMessage.ViewportChange -> throw MessageNeverUsedException()

            // lsp
            is OutputMessage.RequestLspCodeAction -> TODO()
            is OutputMessage.RequestLspDeclaration -> TODO()
            is OutputMessage.RequestLspDefinition -> TODO()
            is OutputMessage.RequestLspDocumentSymbols -> TODO()
            is OutputMessage.RequestLspHover -> TODO()
            is OutputMessage.RequestLspImplementation -> TODO()
            is OutputMessage.RequestLspReferences -> TODO()
            is OutputMessage.RequestLspRename -> TODO()
            is OutputMessage.RequestLspTypeDefinition -> TODO()

            // prompt
            is OutputMessage.PromptOpened -> TODO()

            // editor
            is OutputMessage.ShowInfo -> TODO()
            is OutputMessage.JumpsChanged -> TODO()
            is OutputMessage.MarksChanged -> TODO()
            is OutputMessage.KeyboardLayoutChanged -> TODO()
            is OutputMessage.SyncBufferRequest -> {
                project.serviceAsync<KiBufferManager>().handleSyncBufferRequest(message)
            }

        }
    }

    private suspend fun sendRequestInternal(message: InputMessage): OutputMessage {
        val messageId = nextMessageId++
        val inputWrapper = InputMessageWrapper(message, messageId)

        val outputWrapper = withTimeout(5.seconds) {
            suspendCoroutine { continuation ->
                pendingRequests[inputWrapper.id] = continuation

                scope.launch {
                    webSocketSession.outgoing.send(Frame.Text(KiJson.encodeToString(inputWrapper)))
                }
            }
        }

        val message = outputWrapper.message

        // TODO outputWrapper.error is ignored because it is the exact same information as in the body
        if (message is OutputMessage.Error) {
            throw OutputMessageException(message.params)
        }

        return message
    }

    private suspend fun sendNotificationInternal(message: InputMessage) {
        val messageWrapper = InputMessageWrapper(message, 0u)
        webSocketSession.outgoing.send(Frame.Text(KiJson.encodeToString(messageWrapper)))
    }

    suspend fun sendRequest(message: InputMessage.Ping): OutputMessage.Ping {
        return sendRequestInternal(message) as OutputMessage.Ping
    }

    suspend fun sendNotification(message: InputMessage.BufferOpen) {
        return sendNotificationInternal(message)
    }

    suspend fun sendNotification(message: InputMessage.BufferChange) {
        return sendNotificationInternal(message)
    }

    suspend fun sendNotification(message: InputMessage.BufferActive) {
        return sendNotificationInternal(message)
    }

    suspend fun sendNotification(message: InputMessage.SyncBufferResponse) {
        return sendNotificationInternal(message)
    }

    suspend fun sendNotification(message: InputMessage.SelectionSet) {
        return sendNotificationInternal(message)
    }

    suspend fun sendNotification(message: InputMessage.KeyboardInput) {
        return sendNotificationInternal(message)
    }

    suspend fun sendNotification(message: InputMessage.ViewportChange) {
        return sendNotificationInternal(message)
    }

    suspend fun sendNotification(message: InputMessage.DiagnosticsChange) {
        return sendNotificationInternal(message)
    }

    suspend fun sendNotification(message: InputMessage.PromptEnter) {
        return sendNotificationInternal(message)
    }
}

private val KiJson = Json {
    classDiscriminator = "tag"
}
