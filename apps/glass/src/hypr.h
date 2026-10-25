/* hypr.h: one request to the running Hyprland, over its control socket. */
#ifndef VETRO_GLASS_HYPR_H
#define VETRO_GLASS_HYPR_H

#include <stdbool.h>
#include <stddef.h>

/* Sends `request` ("hyprglass tune reload", as hyprctl would) and puts the
 * answer, cut to `cap` - 1 bytes and ended with a zero, in `reply`. False when
 * no Hyprland is running for this session or the socket does not answer within
 * a second. */
bool hypr_request(const char *request, char *reply, size_t cap);

#endif
