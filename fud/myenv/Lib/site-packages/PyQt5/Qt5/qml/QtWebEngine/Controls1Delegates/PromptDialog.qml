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

// FIXME: prompt missing in Qt Quick Dialogs atm. Make our own for now.
import QtQuick.Controls 1.4
import QtQuick.Layouts 1.0
import QtQuick 2.5

ApplicationWindow {
    signal input(string text)
    signal accepted
    signal rejected
    property alias text: message.text
    property alias prompt: field.text

    width: 350
    height: 100
    flags: Qt.Dialog

    onClosing: {
        rejected();
    }

    function open() {
        show();
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 4
        Text {
            id: message
            Layout.fillWidth: true
        }
        TextField {
            id:field
            Layout.fillWidth: true
        }
        RowLayout {
            Layout.alignment: Qt.AlignRight
            spacing: 8
            Button {
                text: qsTr("OK")
                onClicked: {
                    input(field.text);
                    accepted();
                    close();
                    destroy();
                }
            }
            Button {
                text: qsTr("Cancel")
                onClicked: {
                    rejected();
                    close();
                    destroy();
                }
            }
        }
    }

}
