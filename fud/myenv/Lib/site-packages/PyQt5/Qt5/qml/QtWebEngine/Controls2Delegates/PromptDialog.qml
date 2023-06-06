/****************************************************************************
**
** Copyright (C) 2016 The Qt Company Ltd.
** Contact: https://www.qt.io/licensing/
**
** This file is part of the QtWebEngine module of the Qt Toolkit.
**
** $QT_BEGIN_LICENSE:LGPL$
** Commercial License Usage
** Licensees holding valid commercial Qt licenses may use this file in
** accordance with the commercial license agreement provided with the
** Software or, alternatively, in accordance with the terms contained in
** a written agreement between you and The Qt Company. For licensing terms
** and conditions see https://www.qt.io/terms-conditions. For further
** information use the contact form at https://www.qt.io/contact-us.
**
** GNU Lesser General Public License Usage
** Alternatively, this file may be used under the terms of the GNU Lesser
** General Public License version 3 as published by the Free Software
** Foundation and appearing in the file LICENSE.LGPL3 included in the
** packaging of this file. Please review the following information to
** ensure the GNU Lesser General Public License version 3 requirements
** will be met: https://www.gnu.org/licenses/lgpl-3.0.html.
**
** GNU General Public License Usage
** Alternatively, this file may be used under the terms of the GNU
** General Public License version 2.0 or (at your option) the GNU General
** Public license version 3 or any later version approved by the KDE Free
** Qt Foundation. The licenses are as published by the Free Software
** Foundation and appearing in the file LICENSE.GPL2 and LICENSE.GPL3
** included in the packaging of this file. Please review the following
** information to ensure the GNU General Public License requirements will
** be met: https://www.gnu.org/licenses/gpl-2.0.html and
** https://www.gnu.org/licenses/gpl-3.0.html.
**
** $QT_END_LICENSE$
**
****************************************************************************/

import QtQuick 2.5
import QtQuick.Controls 2.0 as Controls
import QtQuick.Layouts 1.0
import QtQuick.Dialogs 1.2

Dialog {
    property alias text: message.text
    property alias prompt: field.text
    property bool handled: false
    signal input(string text)
    signal accepted()
    signal rejected()
    title: qsTr("Prompt Dialog")
    modality: Qt.NonModal

    //handle the case where users simply closes the dialog
    onVisibilityChanged: {
        if (visible == false && handled == false) {
            handled = true;
            rejected();
        } else {
            handled = false;
        }
    }

    function acceptDialog() {
        input(field.text);
        accepted();
        handled = true;
        close();
    }

    function rejectDialog() {
        rejected();
        handled = true;
        close();
    }

    contentItem: ColumnLayout {
        id: rootLayout
        anchors.fill: parent
        anchors.margins: 4
        property int minimumWidth: rootLayout.implicitWidth + rootLayout.doubleMargins
        property int minimumHeight: rootLayout.implicitHeight + rootLayout.doubleMargins
        property int doubleMargins: anchors.margins * 2
        SystemPalette { id: palette; colorGroup: SystemPalette.Active }
        Text {
            id: message
            Layout.fillWidth: true
            color: palette.windowText
        }
        Controls.TextField {
            id:field
            focus: true
            Layout.fillWidth: true
            onAccepted: acceptDialog()
        }
        Item {
            Layout.fillHeight: true
        }
        RowLayout {
            Layout.alignment: Qt.AlignRight
            spacing: 8
            Controls.Button {
                text: qsTr("OK")
                onClicked: acceptDialog()
            }
            Controls.Button {
                text: qsTr("Cancel")
                onClicked: rejectDialog()
            }
        }
    }

}
