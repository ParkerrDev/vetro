/* hypr.c: one request to the running Hyprland, over its control socket. */
#include "hypr.h"

#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/time.h>
#include <sys/un.h>
#include <unistd.h>

/* The socket of this session's Hyprland, from the variables Hyprland sets. */
static bool socket_path(char *path, size_t cap) {
    const char *runtime = getenv("XDG_RUNTIME_DIR");
    const char *instance = getenv("HYPRLAND_INSTANCE_SIGNATURE");
    if (runtime == NULL || instance == NULL || runtime[0] == '\0' || instance[0] == '\0')
        return false;
    const int written = snprintf(path, cap, "%s/hypr/%s/.socket.sock", runtime, instance);
    return written > 0 && (size_t)written < cap;
}

/* Reads the answer until Hyprland closes the connection, at most cap - 1 bytes. */
static bool read_reply(int fd, char *reply, size_t cap) {
    size_t have = 0;
    /* bounded: every pass reads at least a byte or ends */
    while (have + 1 < cap) {
        const ssize_t got = read(fd, reply + have, cap - 1 - have);
        if (got < 0)
            return false;
        if (got == 0)
            break;
        have += (size_t)got;
    }
    reply[have] = '\0';
    return true;
}

bool hypr_request(const char *request, char *reply, size_t cap) {
    assert(request != NULL && reply != NULL && cap > 0);
    reply[0] = '\0';

    struct sockaddr_un address = {.sun_family = AF_UNIX};
    if (!socket_path(address.sun_path, sizeof(address.sun_path)))
        return false;

    const int fd = socket(AF_UNIX, SOCK_STREAM | SOCK_CLOEXEC, 0);
    if (fd < 0)
        return false;
    const struct timeval second = {.tv_sec = 1, .tv_usec = 0};
    bool ok = setsockopt(fd, SOL_SOCKET, SO_RCVTIMEO, &second, sizeof(second)) == 0 &&
              setsockopt(fd, SOL_SOCKET, SO_SNDTIMEO, &second, sizeof(second)) == 0 &&
              connect(fd, (const struct sockaddr *)&address, sizeof(address)) == 0;
    const size_t length = strlen(request);
    ok = ok && write(fd, request, length) == (ssize_t)length;
    ok = ok && read_reply(fd, reply, cap);
    close(fd);
    return ok;
}
