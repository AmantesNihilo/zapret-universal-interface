package io.github.valnesfjord.tgwsproxyrs

/**
 * One line of proxy output, tagged with an id that is monotonic for the life
 * of the process.
 *
 * The id exists for the log `LazyColumn`: the buffer drops its oldest line
 * once it is full, so a positional key would change for every row on screen
 * every time a line arrives and force a full relayout instead of a one-row
 * append.
 */
data class LogLine(val id: Long, val text: String)
