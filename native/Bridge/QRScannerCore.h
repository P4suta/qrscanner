#ifndef QRSCANNER_CORE_H
#define QRSCANNER_CORE_H

#include <stdint.h>

#define QR_PHASE_IDLE 0U
#define QR_PHASE_SCANNING 1U
#define QR_PHASE_RESULTS 2U
#define QR_PHASE_OPENING 3U
#define QR_PHASE_EMPTY 4U
#define QR_PHASE_FAILED 5U

#define QR_EVENT_BEGIN 0U
#define QR_EVENT_CANCEL 1U
#define QR_EVENT_COMPLETED 2U
#define QR_EVENT_FAILED 3U
#define QR_EVENT_OPEN 4U
#define QR_EVENT_COPY 5U
#define QR_EVENT_OPEN_FINISHED 6U

#define QR_EFFECT_NONE 0U
#define QR_EFFECT_CAPTURE 1U
#define QR_EFFECT_RESULTS 2U
#define QR_EFFECT_EMPTY 3U
#define QR_EFFECT_FAILURE 4U
#define QR_EFFECT_OPEN 5U
#define QR_EFFECT_COPY 6U
#define QR_EFFECT_CLOSE 7U
#define QR_EFFECT_OPEN_FAILURE 8U
#define QR_EFFECT_EXHAUSTED 9U

typedef struct {
    uint64_t generation;
    uint32_t phase;
} QRScanState;

typedef struct {
    uint64_t generation;
    uint32_t tag;
    uint32_t value;
} QRScanEvent;

typedef struct {
    QRScanState state;
    uint32_t effect;
    uint32_t status;
} QRScanTransition;

_Static_assert(sizeof(QRScanState) == 16, "QRScanState ABI");
_Static_assert(sizeof(QRScanEvent) == 16, "QRScanEvent ABI");
_Static_assert(sizeof(QRScanTransition) == 24, "QRScanTransition ABI");

QRScanTransition qr_transition(QRScanState state, QRScanEvent event);

#endif
