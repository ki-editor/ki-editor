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
import kotlinx.serialization.json.Json
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
        val kiBinary = System.getProperty("ki.binary")
            ?: "ki"

        val process = ProcessBuilder(kiBinary, "@", "embed", ".")
            .redirectErrorStream(true)
            .start()

        val deferredPort = CompletableDeferred<Int>()
        val portRegex = "^KI_LISTENING_ON=(.+)$".toRegex()

        scope.launch {
            val reader = process.inputReader()

            while (true) {
                val line = withContext(Dispatchers.IO) { reader.readLine() }

                logger.debug("Ki Editor: $line")

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

                    logger.debug("Received text from ws connection: $messageWrapper")

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

    private fun handleOutputMessage(wrapper: OutputMessageWrapper) {
        if (wrapper.id == 0u) {
            scope.launch {
                handleNotification(wrapper.message)
            }
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
            is OutputMessage.Error -> throw OutputMessageException(message.params)

            // buffer
            is OutputMessage.BufferDiff -> project.serviceAsync<KiNotificationHandler>().handleBufferDiff(message)
            is OutputMessage.BufferOpen -> throw MessageNeverUsedException()
            is OutputMessage.BufferSave -> project.serviceAsync<KiNotificationHandler>().handleBufferSave(message)

            // selection
            is OutputMessage.SelectionUpdate -> {
                project.serviceAsync<KiNotificationHandler>().handleSelectionUpdate(message)
            }

            // mode
            is OutputMessage.ModeChange -> project.serviceAsync<KiNotificationHandler>().handleModeChange(message)
            is OutputMessage.SelectionModeChange -> {
                project.serviceAsync<KiNotificationHandler>().handleSelectionModeChange(message)
            }

            // viewport
            is OutputMessage.ViewportChange -> throw MessageNeverUsedException()

            // lsp
            is OutputMessage.RequestLspCodeAction -> {
                project.serviceAsync<KiEditorLspBridge>().handleLspCodeAction()
            }

            is OutputMessage.RequestLspDeclaration -> {
                project.serviceAsync<KiEditorLspBridge>().handleLspDeclaration()
            }

            is OutputMessage.RequestLspDefinition -> {
                project.serviceAsync<KiEditorLspBridge>().handleLspDefinition()
            }

            is OutputMessage.RequestLspDocumentSymbols -> {
                project.serviceAsync<KiEditorLspBridge>().handleLspDocumentSymbols()
            }

            is OutputMessage.RequestLspHover -> {
                project.serviceAsync<KiEditorLspBridge>().handleLspHover()
            }

            is OutputMessage.RequestLspImplementation -> {
                project.serviceAsync<KiEditorLspBridge>().handleLspImplementation()
            }

            is OutputMessage.RequestLspReferences -> {
                project.serviceAsync<KiEditorLspBridge>().handleLspReferences()
            }

            is OutputMessage.RequestLspRename -> {
                project.serviceAsync<KiEditorLspBridge>().handleLspRename()
            }

            is OutputMessage.RequestLspTypeDefinition -> {
                project.serviceAsync<KiEditorLspBridge>().handleLspTypeDefinition()
            }

            // prompt
            is OutputMessage.PromptOpened -> {
                // todo handle this
            }

            // editor
            is OutputMessage.ShowInfo -> {
                // todo handle this
            }

            is OutputMessage.JumpsChanged -> project.serviceAsync<KiNotificationHandler>().handleJumpsChanged(message)
            is OutputMessage.MarksChanged -> project.serviceAsync<KiNotificationHandler>().handleMarksChanged(message)

            is OutputMessage.KeyboardLayoutChanged -> {
                project.serviceAsync<KiNotificationHandler>().handleKeyboardLayoutChange(message)
            }

            is OutputMessage.SyncBufferRequest -> {
                project.serviceAsync<KiNotificationHandler>().handleSyncBufferRequest(message)
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

    suspend fun sendRequest(message: InputMessage.BufferOpen) {
        // TODO even though it is a request because it can return error,
        //      in a happy path no response is sent, so use notification for now
        return sendNotificationInternal(message)
    }

    suspend fun sendRequest(message: InputMessage.DiagnosticsChange) {
        // TODO same as above
        return sendNotificationInternal(message)
    }

    suspend fun sendRequest(message: InputMessage.KeyboardInput) {
        // TODO same as above
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

    suspend fun sendNotification(message: InputMessage.ViewportChange) {
        return sendNotificationInternal(message)
    }

    suspend fun sendNotification(message: InputMessage.PromptEnter) {
        return sendNotificationInternal(message)
    }
}

private val KiJson = Json {
    classDiscriminator = "tag"
}
