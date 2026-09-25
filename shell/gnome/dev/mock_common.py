from datetime import timedelta, timezone

SECOND = timedelta(seconds=1)
MINUTE = timedelta(minutes=1)
HOUR = timedelta(hours=1)
DAY = timedelta(days=1)
TONE_RANK = {"neutral": 0, "good": 1, "warning": 2, "critical": 3}


def iso(moment):
    return moment.astimezone(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ") if moment else None


def worst_tone(tones):
    return max(tones, key=TONE_RANK.__getitem__, default=None)
