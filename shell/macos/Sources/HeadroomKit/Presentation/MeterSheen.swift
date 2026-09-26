public enum MeterSheen {
    public static let sweepSeconds = 1.4
    public static let restSeconds = 5.0
    public static let frameSeconds = 1.0 / 30
    public static let startDelaySeconds = 0.6
    public static let minimumFill = 0.03
    public static let bandWidth = 36.0
    public static let peakOpacity = 0.32
    static let epsilon = 1e-6

    public static var cycleSeconds: Double { sweepSeconds + restSeconds }

    public static func runs(fill: Double, reducedMotion: Bool) -> Bool {
        !reducedMotion && fill.isFinite && fill >= minimumFill
    }

    public static func progress(elapsed: Double) -> Double? {
        guard elapsed.isFinite, elapsed >= 0 else { return nil }
        let phase = max(0, elapsed - cycleStart(containing: elapsed))
        guard phase < sweepSeconds else { return nil }
        let linear = phase / sweepSeconds
        return linear * linear * (3 - 2 * linear)
    }

    public static func nextFrame(after elapsed: Double, frameSeconds: Double = frameSeconds) -> Double {
        guard elapsed.isFinite, elapsed >= 0 else { return 0 }
        let cycleStart = cycleStart(containing: elapsed)
        let sweepEnd = cycleStart + sweepSeconds
        guard elapsed + epsilon < sweepEnd, frameSeconds > 0 else { return cycleStart + cycleSeconds }
        return min(elapsed + frameSeconds, sweepEnd)
    }

    static func cycleStart(containing elapsed: Double) -> Double {
        let start = (elapsed / cycleSeconds).rounded(.down) * cycleSeconds
        return start + cycleSeconds <= elapsed + epsilon ? start + cycleSeconds : start
    }

    public static func bandOffset(progress: Double, fillWidth: Double, bandWidth: Double = bandWidth) -> Double {
        -bandWidth + (fillWidth + bandWidth) * min(1, max(0, progress))
    }
}
