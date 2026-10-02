fun port(env: Map<String, String>): Int =
    env["PORT"]?.toIntOrNull() ?: throw IllegalArgumentException("PORT must be a number")
