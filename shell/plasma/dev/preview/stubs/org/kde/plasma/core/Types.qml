pragma Singleton

import QtQuick

QtObject {
    enum FormFactor {
        Planar,
        MediaCenter,
        Horizontal,
        Vertical,
        Application
    }

    enum ItemStatus {
        UnknownStatus,
        PassiveStatus,
        ActiveStatus,
        NeedsAttentionStatus
    }
}
