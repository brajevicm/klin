fun port(env: Map<String, String>): Int = runCatching { env.getValue("PORT").toInt() }.getOrDefault(0)
