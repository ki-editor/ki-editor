package com.kieditor.exceptions

class OutputMessageException(error: String) : RuntimeException("Ki editor returned an error: $error")
