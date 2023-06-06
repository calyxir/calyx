/****************************************************************************
**
** Copyright (C) 2018 The Qt Company Ltd.
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
import QtQuick.Layouts 1.3

Rectangle {
    id: menu

    signal cutTriggered
    signal copyTriggered
    signal pasteTriggered
    signal contextMenuTriggered

    property bool isCutEnabled: false
    property bool isCopyEnabled: false
    property bool isPasteEnabled: false

    property color borderColor: "darkGray"
    property color bgColor: "white"

    radius: 4
    border.color: borderColor
    color: borderColor
    antialiasing: true

    RowLayout {
        anchors.fill: parent
        spacing: parent.border.width
        anchors.margins: parent.border.width

        Rectangle {
            Layout.fillHeight: true
            Layout.fillWidth: true
            radius: menu.radius
            color: bgColor
            visible: isCutEnabled

            Text {
                id: cutText
                anchors.centerIn: parent
                text: "Cut"
            }

            MouseArea {
                anchors.fill: parent
                onPressed: {
                    parent.color = borderColor;
                    cutText.color = "white";
                }
                onReleased: {
                    parent.color = bgColor;
                    cutText.color = "black";
                    cutTriggered();
                }
            }
        }

        Rectangle {
            Layout.fillHeight: true
            Layout.fillWidth: true
            radius: menu.radius
            color: bgColor
            visible: isCopyEnabled

            Text {
                id: copyText
                anchors.centerIn: parent
                text: "Copy"
            }

            MouseArea {
                anchors.fill: parent
                onPressed: {
                    parent.color = borderColor;
                    copyText.color = "white";
                }
                onReleased: {
                    parent.color = bgColor;
                    copyText.color = "black";
                    copyTriggered();
                }
            }
        }

        Rectangle {
            Layout.fillHeight: true
            Layout.fillWidth: true
            radius: menu.radius
            color: bgColor
            visible: isPasteEnabled

            Text {
                id: pasteText
                anchors.centerIn: parent
                text: "Paste"
            }

            MouseArea {
                anchors.fill: parent
                onPressed: {
                    parent.color = borderColor;
                    pasteText.color = "white";
                }
                onReleased: {
                    parent.color = bgColor;
                    pasteText.color = "black";
                    pasteTriggered();
                }
            }
        }

        Rectangle {
            Layout.fillHeight: true
            Layout.fillWidth: true
            radius: menu.radius
            color: bgColor

            Text {
                id: contextMenuText
                anchors.centerIn: parent
                text: "..."
            }

            MouseArea {
                anchors.fill: parent
                onPressed: {
                    parent.color = borderColor;
                    contextMenuText.color = "white";
                }
                onReleased: {
                    parent.color = bgColor;
                    contextMenuText.color = "black";
                    contextMenuTriggered();
                }
            }
        }
    }
}
