/**********************************************************
 * MIDI 2.0 Library 
 * Author: Andrew Mee
 * 
 * MIT License
 * Copyright 2021 Andrew Mee
 * 
 * Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:
 * 
 * The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.
 * 
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 * 
 * ********************************************************/

#include "../include/umpProcessor.h"
#include "../include/utils.h"
#include <algorithm>


void umpProcessor::clearUMP(){
	messPos =0;
}

void umpProcessor::processUMP(uint32_t UMP){
	uint8_t mt = (UMP >> 28) & 0xF;
	umpMess[messPos] = UMP;
	uint8_t group = (umpMess[0] >> 24) & 0xF;
    void * refpoint = this->refpoint;

	if(mt <= 0x1){
		if(utilityMessage != nullptr){
			umpGeneric mess = umpGeneric();
			mess.refpoint = refpoint;
			mess.umpGroup = group;
			mess.messageType = mt;
			mess.status = (umpMess[0] >> 20) & 0xF;
			mess.value = umpMess[0] & 0xFFFF;
			utilityMessage(mess);
		}
		messPos =0;
	} else if(mt == 0x2){
		if(systemMessage != nullptr){
			umpGeneric mess = umpGeneric();
			mess.refpoint = refpoint;
			mess.umpGroup = group;
			mess.messageType = mt;
			mess.status = (umpMess[0] >> 16) & 0xFF;
			mess.value = ((umpMess[0] >> 8) & 0x7F) + ((umpMess[0] & 0x7F) << 7);
			systemMessage(mess);
		}
		messPos =0;
	} else if(mt == 0x3){
		if(channelVoiceMessage != nullptr){
			umpCVM mess = umpCVM();
			mess.refpoint = refpoint;
			mess.umpGroup = group;
			mess.messageType = mt;
			mess.status = (umpMess[0] >> 16) & 0xF0;
			mess.channel = (umpMess[0] >> 16) & 0xF;
			mess.note = (umpMess[0] >> 8) & 0x7F;
			mess.value = umpMess[0] & 0x7F;
			channelVoiceMessage(mess);
		}
		messPos =0;
	} else if(mt == 0x4){
		if(channelVoiceMessage != nullptr){
			umpCVM mess = umpCVM();
			mess.refpoint = refpoint;
			mess.umpGroup = group;
			mess.messageType = mt;
			mess.status = (umpMess[0] >> 16) & 0xF0;
			mess.channel = (umpMess[0] >> 16) & 0xF;
			mess.note = (umpMess[0] >> 8) & 0x7F;
			mess.index = umpMess[0] & 0xFF;
			mess.value = umpMess[1];
			channelVoiceMessage(mess);
		}
		messPos =0;
	} else if(mt == 0x5 && messPos == 3){
        if(mt == 0x5){
            uint8_t status = (umpMess[0] >> 20) & 0xF;
            if(status <= 3){
                umpData mess = umpData();
                mess.refpoint = refpoint;
                mess.umpGroup = group;
                mess.messageType = mt;
                mess.streamId  = (umpMess[0] >> 8) & 0xFF;
                mess.form = status;
                mess.dataLength  = (uint8_t)std::min((uint8_t)(umpMess[0] >> 16) & 0xF, 13);
                M2Utils::clear(callbackBuffer, 0, sizeof(callbackBuffer));

                if(mess.dataLength >= 1)callbackBuffer[0] =  umpMess[0] & 0xFF;
                if(mess.dataLength >= 2)callbackBuffer[1] =  (umpMess[1] >> 24) & 0xFF;
                if(mess.dataLength >= 3)callbackBuffer[2] =  (umpMess[1] >> 16) & 0xFF;
                if(mess.dataLength >= 4)callbackBuffer[3] =  (umpMess[1] >> 8) & 0xFF;
                if(mess.dataLength >= 5)callbackBuffer[4] =  umpMess[1] & 0xFF;
                if(mess.dataLength >= 6)callbackBuffer[5] =  (umpMess[2] >> 24) & 0xFF;
                if(mess.dataLength >= 7)callbackBuffer[6] =  (umpMess[2] >> 16) & 0xFF;
                if(mess.dataLength >= 8)callbackBuffer[7] =  (umpMess[2] >> 8) & 0xFF;
                if(mess.dataLength >= 9)callbackBuffer[8] =  umpMess[2] & 0xFF;
                if(mess.dataLength >= 10)callbackBuffer[9] =  (umpMess[3] >> 24) & 0xFF;
                if(mess.dataLength >= 11)callbackBuffer[10] =  (umpMess[3] >> 16) & 0xFF;
                if(mess.dataLength >= 12)callbackBuffer[11] =  (umpMess[3] >> 8) & 0xFF;
                if(mess.dataLength >= 13)callbackBuffer[12] =  umpMess[3] & 0xFF;

                mess.data = callbackBuffer;
                sendOutSysex(mess);
                M2Utils::clear(callbackBuffer, 0, sizeof(callbackBuffer));

            }else if(status == 8){ //MDS Header

                if(mds5Header)mds5Header(
                    group,
                    (umpMess[1] >> 16) & 0xF,
                    umpMess[0] & 0xFFFF,
                    (umpMess[1] >> 16) & 0xFFFF,
                    umpMess[1] & 0xFFFF,
                    (umpMess[2] >> 16) & 0xFFFF,
                    umpMess[2] & 0xFFFF,
                    (umpMess[3] >> 16) & 0xFFFF,
                    umpMess[3] & 0xFFFF
                    );
            }else if(status == 9){ //MDS Payload
                M2Utils::clear(callbackBuffer, 0, sizeof(callbackBuffer));
                callbackBuffer[0] =  (umpMess[0] >> 8) & 0xFF;
                callbackBuffer[1] =  umpMess[0] & 0xFF;
                callbackBuffer[2] =  (umpMess[1] >> 24) & 0xFF;
                callbackBuffer[3] =  (umpMess[1] >> 16) & 0xFF;
                callbackBuffer[4] =  (umpMess[1] >> 8) & 0xFF;
                callbackBuffer[5] =  umpMess[1] & 0xFF;
                callbackBuffer[6] =  (umpMess[2] >> 24) & 0xFF;
                callbackBuffer[7] =  (umpMess[2] >> 16) & 0xFF;
                callbackBuffer[8] =  (umpMess[2] >> 8) & 0xFF;
                callbackBuffer[9] =  umpMess[2] & 0xFF;
                callbackBuffer[10] =  (umpMess[3] >> 24) & 0xFF;
                callbackBuffer[11] =  (umpMess[3] >> 16) & 0xFF;
                callbackBuffer[12] =  (umpMess[3] >> 8) & 0xFF;
                callbackBuffer[13] =  umpMess[3] & 0xFF;
                if(mds5Payload)mds5Payload(
                    group,
                    (umpMess[1] >> 16) & 0xF,
                    callbackBuffer, 14
                    );
                M2Utils::clear(callbackBuffer, 0, sizeof(callbackBuffer));
            }else {
                if(unknownUMPMessage)unknownUMPMessage(umpMess, 4);
            }

        }
		messPos =0;
	} else if(mt == 0xD && messPos == 3){
        if(mt == 0xD){
            umpFlexData mess = umpFlexData();
            mess.refpoint = refpoint;
            mess.umpGroup = group;
            mess.channel = (umpMess[0] >> 16) & 0xF;
            mess.messageType = mt;
            mess.status = umpMess[0] & 0xFF;
            mess.statusBank = (umpMess[0] >> 8) & 0xFF;
            mess.form = (umpMess[0] >> 22) & 3;
            mess.addrs = (umpMess[0] >> 20) & 3;
            mess.data = umpMess;

            //SysEx 8
            switch (mess.statusBank){
                case FLEXDATA_COMMON:{ //Common/Configuration for MIDI File, Project, and Track
                    switch (mess.status){
                        case FLEXDATA_COMMON_TEMPO: { //Set Tempo Message
                            if(flexTempo != nullptr) flexTempo(mess, umpMess[1]);
                            else if (flexData != nullptr) flexData(mess);
                            break;
                        }
                        case FLEXDATA_COMMON_TIMESIG: { //Set Time Signature Message
                            if(flexTimeSig != nullptr) flexTimeSig(mess,
                                                                 (umpMess[1] >> 24) & 0xFF,
                                                                 (umpMess[1] >> 16) & 0xFF,
                                                                 (umpMess[1] >> 8) & 0xFF
                                   );
                            else if (flexData != nullptr) flexData(mess);
                            break;
                        }
                        case FLEXDATA_COMMON_METRONOME: { //Set Metronome Message
                            if(flexMetronome != nullptr) flexMetronome(mess,
                                                                   (umpMess[1] >> 24) & 0xFF,
                                                                   (umpMess[1] >> 16) & 0xFF,
                                                                   (umpMess[1] >> 8) & 0xFF,
                                                                   umpMess[1] & 0xFF,
                                                                   (umpMess[2] >> 24) & 0xFF,
                                                                   (umpMess[2] >> 16) & 0xFF
                                );
                            else if (flexData != nullptr) flexData(mess);
                            break;
                        }
                        case FLEXDATA_COMMON_KEYSIG: { //Set Key Signature Message
                            if(flexKeySig != nullptr) flexKeySig(mess,
                                                                   (umpMess[1] >> 24) & 0xFF,
                                                                   (umpMess[1] >> 16) & 0xFF
                                );
                            else if (flexData != nullptr) flexData(mess);
                            break;
                        }
                        case FLEXDATA_COMMON_CHORD: { //Set Chord Message
                            if(flexChord != nullptr) flexChord(mess,
                                                                       (umpMess[1] >> 28) & 0xF, //chShrpFlt
                                                                       (umpMess[1] >> 24) & 0xF, //chTonic
                                                                       (umpMess[1] >> 16) & 0xFF, //chType
                                                                       (umpMess[1] >> 12) & 0xF, //chAlt1Type
                                                                       (umpMess[1] >> 8) & 0xF,//chAlt1Deg
                                                                       (umpMess[1] >> 4) & 0xF,//chAlt2Type
                                                                       umpMess[1] & 0xF,//chAlt2Deg
                                                                       (umpMess[2] >> 28) & 0xF,//chAlt3Type
                                                                       (umpMess[2] >> 24) & 0xF,//chAlt3Deg
                                                                       (umpMess[2] >> 20) & 0xF,//chAlt4Type
                                                                       (umpMess[2] >> 16) & 0xF,//chAlt4Deg
                                                                       (umpMess[3] >> 28) & 0xF,//baShrpFlt
                                                                    (umpMess[3] >> 24) & 0xF,//baTonic
                                                                (umpMess[3] >> 16) & 0xFF,//baType
                                                               (umpMess[3] >> 12) & 0xF,//baAlt1Type
                                                               (umpMess[3] >> 8) & 0xF,//baAlt1Deg
                                                               (umpMess[3] >> 4) & 0xF,//baAlt2Type
                                                               umpMess[3] & 0xF//baAlt2Deg
                                );
                            else if (flexData != nullptr) flexData(mess);
                            break;
                        }
                        default:
                            if(flexData != nullptr) {
                                flexData(mess);
                            }
                            break;
                    }
                    break;
                }
                case FLEXDATA_PERFORMANCE: //Performance Events
                case FLEXDATA_LYRIC:{ //Lyric Events
                        uint8_t dataLength  = 0;
                        M2Utils::clear(callbackBuffer, 0, sizeof(callbackBuffer));

                        auto push_byte = [&](uint8_t c) {
                            if (c && dataLength < sizeof(callbackBuffer)) callbackBuffer[dataLength++] = c;
                        };

                        push_byte((umpMess[1] >> 24) & 0xFF);
                        push_byte((umpMess[1] >> 16) & 0xFF);
                        push_byte((umpMess[1] >> 8) & 0xFF);
                        push_byte(umpMess[1] & 0xFF);
                        push_byte((umpMess[2] >> 24) & 0xFF);
                        push_byte((umpMess[2] >> 16) & 0xFF);
                        push_byte((umpMess[2] >> 8) & 0xFF);
                        push_byte(umpMess[2] & 0xFF);
                        push_byte((umpMess[3] >> 24) & 0xFF);
                        push_byte((umpMess[3] >> 16) & 0xFF);
                        push_byte((umpMess[3] >> 8) & 0xFF);
                        push_byte(umpMess[3] & 0xFF);

                        if(mess.statusBank== FLEXDATA_LYRIC && flexLyric != nullptr) flexLyric(mess, callbackBuffer, dataLength);
                        else if(mess.statusBank== FLEXDATA_PERFORMANCE && flexPerformance != nullptr) flexPerformance(mess, callbackBuffer, dataLength);
                        else if (flexData != nullptr) flexData(mess);
                        M2Utils::clear(callbackBuffer, 0, sizeof(callbackBuffer));
                    break;
                }
                default:
                    if(flexData != nullptr) {
                        flexData(mess);
                    }
                    break;
            }
        }
		messPos =0;
	} else if(mt == 0xF && messPos == 3){
        if(mt == 0xF){
            uint16_t status = (umpMess[0] >> 16) & 0x3FF;

            switch (status){
                case MIDIENDPOINT:
                    if(midiEndpoint != nullptr)
                        midiEndpoint((uint8_t) (umpMess[0] >> 8),
                                     (uint8_t) umpMess[0],
                                     (uint8_t) (umpMess[0] >> 10) & 1
                                     );
                    break;
                case MIDIENDPOINT_INFO_NOTIFICATION:
                    if(midiEndpointInfo != nullptr)
                        midiEndpointInfo((uint8_t) (umpMess[0] >> 8),
                                (uint8_t) umpMess[0],  //Min Ver
                                (umpMess[1]>>24) & 0xFF, //Num Of Func Block
                                ((umpMess[1]>>9) & 0x1), //M2 Support
                                ((umpMess[1]>>8) & 0x1), //M1 Support
                                ((umpMess[1]>>1) & 0x1), //rxjr Support
                                (umpMess[1] & 0x1) //txjr Support
                                );
                    break;

                case MIDIENDPOINT_DEVICEINFO_NOTIFICATION:
                    if(midiEndpointDeviceInfo != nullptr) {
                        midiEndpointDeviceInfo(
                                {(uint8_t)((umpMess[1] >> 16) & 0x7F),(uint8_t)((umpMess[1] >> 8) & 0x7F), (uint8_t)(umpMess[1] & 0x7F)},
                                {(uint8_t)((umpMess[2] >> 24) & 0x7F) , (uint8_t)((umpMess[2] >> 16) & 0x7F)},
                                {(uint8_t)((umpMess[2] >> 8) & 0x7F ), (uint8_t)(umpMess[2]  & 0x7F)},
                                {(uint8_t)((umpMess[3] >> 24) & 0x7F), (uint8_t)((umpMess[3] >> 16) & 0x7F),
                                 (uint8_t)( (umpMess[3] >> 8) & 0x7F), (uint8_t)(umpMess[3] & 0x7F)}
                        );
                    }
                    break;
                case MIDIENDPOINT_NAME_NOTIFICATION:
                case MIDIENDPOINT_PRODID_NOTIFICATION: {
                        umpData mess = umpData();
                        mess.refpoint = refpoint;
                        mess.messageType = mt;
                        mess.status = (uint8_t) status;
                        mess.form = umpMess[0] >> 24 & 0x3;
                        mess.dataLength  = 0;
                        M2Utils::clear(callbackBuffer, 0, sizeof(callbackBuffer));

                        auto push_byte = [&](uint8_t c) {
                            if (c && mess.dataLength < sizeof(callbackBuffer)) callbackBuffer[mess.dataLength++] = c;
                        };

                        push_byte((umpMess[0] >> 8) & 0xFF);
                        push_byte(umpMess[0] & 0xFF);
                        push_byte((umpMess[1] >> 24) & 0xFF);
                        push_byte((umpMess[1] >> 16) & 0xFF);
                        push_byte((umpMess[1] >> 8) & 0xFF);
                        push_byte(umpMess[1] & 0xFF);
                        push_byte((umpMess[2] >> 24) & 0xFF);
                        push_byte((umpMess[2] >> 16) & 0xFF);
                        push_byte((umpMess[2] >> 8) & 0xFF);
                        push_byte(umpMess[2] & 0xFF);
                        push_byte((umpMess[3] >> 24) & 0xFF);
                        push_byte((umpMess[3] >> 16) & 0xFF);
                        push_byte((umpMess[3] >> 8) & 0xFF);
                        push_byte(umpMess[3] & 0xFF);

                        mess.data = callbackBuffer;
                        if(status == MIDIENDPOINT_NAME_NOTIFICATION && midiEndpointName != nullptr) midiEndpointName(mess);
                        if(status == MIDIENDPOINT_PRODID_NOTIFICATION && midiEndpointProdId != nullptr) midiEndpointProdId(mess);
                        M2Utils::clear(callbackBuffer, 0, sizeof(callbackBuffer));
                    break;
                }

                case MIDIENDPOINT_STREAMCONFIG_REQUEST: //JR Protocol Req
                    if(midiEndpointStreamConfigReq != nullptr)
                        midiEndpointStreamConfigReq((uint8_t) (umpMess[0] >> 8),
                                                   (umpMess[0] >> 1) & 1,
                                                   umpMess[0] & 1
                                                   );
                    break;
                case MIDIENDPOINT_STREAMCONFIG_NOTIFICATION: //JR Protocol Req
                    if(midiEndpointStreamConfigNotify != nullptr)
                        midiEndpointStreamConfigNotify((uint8_t) (umpMess[0] >> 8),
                                                     (umpMess[0] >> 1) & 1,
                                                     umpMess[0] & 1
                                                    );
                    break;

                case FUNCTIONBLOCK:{
                    uint8_t filter = umpMess[0] & 0xFF;
                    uint8_t fbIdx = (umpMess[0] >> 8) & 0xFF;
                    if(functionBlock != nullptr) functionBlock(fbIdx, filter);
                    break;
                }

                case FUNCTIONBLOCK_INFO_NOTFICATION:
                    if(functionBlockInfo != nullptr) {
                        uint8_t fbIdx = (umpMess[0] >> 8) & 0x7F;
                        functionBlockInfo(
                                fbIdx, //fbIdx
                                (umpMess[0] >> 15) & 0x1, // active
                                umpMess[0] & 0x3, //dir
                                (umpMess[0] >> 7) & 0x1, // Sender
                                (umpMess[0] >> 6) & 0x1, // Receiver
                                ((umpMess[1] >> 24) & 0x1F), //first group
                                ((umpMess[1] >> 16) & 0x1F), // group length
                                ((umpMess[1] >> 8) & 0x7F), //midiCIVersion
                                ((umpMess[0]>>2)  & 0x3), //isMIDI 1
                                (umpMess[1]  & 0xFF) // max Streams
                        );
                    }
                    break;
                case FUNCTIONBLOCK_NAME_NOTIFICATION:{
                    uint8_t fbIdx = (umpMess[0] >> 8) & 0x7F;
                    umpData mess = umpData();
                    mess.refpoint = refpoint;
                    mess.messageType = mt;
                    mess.status = (uint8_t) status;
                    mess.form = umpMess[0] >> 24 & 0x3;
                    mess.dataLength  = 0;
                    M2Utils::clear(callbackBuffer, 0, sizeof(callbackBuffer));

                    auto push_byte = [&](uint8_t c) {
                        if (c && mess.dataLength < sizeof(callbackBuffer)) callbackBuffer[mess.dataLength++] = c;
                    };

                    push_byte(umpMess[0] & 0xFF);
                    push_byte((umpMess[1] >> 24) & 0xFF);
                    push_byte((umpMess[1] >> 16) & 0xFF);
                    push_byte((umpMess[1] >> 8) & 0xFF);
                    push_byte(umpMess[1] & 0xFF);
                    push_byte((umpMess[2] >> 24) & 0xFF);
                    push_byte((umpMess[2] >> 16) & 0xFF);
                    push_byte((umpMess[2] >> 8) & 0xFF);
                    push_byte(umpMess[2] & 0xFF);
                    push_byte((umpMess[3] >> 24) & 0xFF);
                    push_byte((umpMess[3] >> 16) & 0xFF);
                    push_byte((umpMess[3] >> 8) & 0xFF);
                    push_byte(umpMess[3] & 0xFF);

                    mess.data = callbackBuffer;

                    if(functionBlockName != nullptr) functionBlockName(mess,fbIdx);
                    M2Utils::clear(callbackBuffer, 0, sizeof(callbackBuffer));
                    break;
                }
                case STARTOFSEQ: {
                    if(startOfSeq != nullptr) startOfSeq();
                    break;
                }
                case ENDOFFILE: {
                    if(endOfFile != nullptr) endOfFile();
                    break;
                }
                default:
                    if(unknownUMPMessage)unknownUMPMessage(umpMess, 4);
                    break;

            }
        }
		messPos =0;
	} else if(mt == 0x0 || mt == 0x1 || mt == 0x2 || mt == 0x3 || mt == 0x4){
        //Handled above
    } else {
		messPos++;
	}
}
