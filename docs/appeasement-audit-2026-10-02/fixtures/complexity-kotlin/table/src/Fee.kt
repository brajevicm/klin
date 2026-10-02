fun base(): Int = 5

private val REGION = mapOf("EU" to 4, "US" to 6, "APAC" to 9, "LATAM" to 7, "MEA" to 8, "NORDIC" to 5, "UK" to 3, "CA" to 2, "JP" to 11, "AU" to 10, "IN" to 1)

fun regionFee(r: String): Int = REGION[r] ?: 12
