Filter in sentry init only
===========================

4XX: filtered + sent
Others: sent + sent
Tracing: sent


Filter in middleware only
==========================

4XX: sent + filtered
Others: sent + sent
Tracing: sent

Filter in both places
======================

4XX: filtered + filtered
Others: sent + sent
Tracing: sent

where tracing = explicit `event!`.

