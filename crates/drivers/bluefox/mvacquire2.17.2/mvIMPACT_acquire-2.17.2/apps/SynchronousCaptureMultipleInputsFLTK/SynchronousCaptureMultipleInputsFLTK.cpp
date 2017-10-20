#if !defined(linux) && !defined(__linux) && !defined(__linux__)
#   error Sorry! Linux only code!
#endif // #if !defined(linux) && !defined(__linux) && !defined(__linux__)
#include <FL/Fl.H>
#include <FL/Fl_Window.H>
#include <FL/Fl_Overlay_Window.H>
#include <FL/Fl_Gl_Window.H>
#include <FL/gl.h>
#include <FL/fl_draw.H>
#include <stdio.h>
#include <cstdlib>
#include <iostream>
#include <set>
#include <vector>
#include <apps/Common/exampleHelper.h>
#include <mvIMPACT_CPP/mvIMPACT_acquire.h>

using namespace std;
using namespace mvIMPACT::acquire;

static bool g_boTerminated = false;

#if HAVE_GL
//-----------------------------------------------------------------------------
class MyWindow : public Fl_Gl_Window
    //-----------------------------------------------------------------------------
{
    const Request* pRequest_;
    string msg_;
    virtual void draw()
    {
        if( !valid() )
        {
            valid( 1 );
            glLoadIdentity();
            glViewport( -w(), -h(), 2 * w(), 2 * h() );
        }

        if( pRequest_ )
        {

            gl_draw_image( ( unsigned char* ) pRequest_->imageData.read(), 0, 0,
                           pRequest_->imageWidth.read(), pRequest_->imageHeight.read(),
                           pRequest_->imageBytesPerPixel.read(),
                           pRequest_->imageLinePitch.read() );
        }
    }
    virtual void draw_overlay()
    {
        if ( !valid() )
        {
            valid( 1 );
            glLoadIdentity();
            glViewport( -w(), -h(), 2 * w(), 2 * h() );
        }
        gl_color( FL_RED );
        gl_draw( msg_.c_str(), 8, h() - 20 );
    }
public:
    MyWindow( int W, int H ) : Fl_Gl_Window( W, H ), pRequest_( 0 ), msg_( "" ) {}
    void NewRequest( const Request* pRequest )
    {
        pRequest_ = pRequest;
    }
    void setOverlayString( const string& msg )
    {
        msg_ = msg;
        redraw_overlay();
    }
};

//-----------------------------------------------------------------------------
MyWindow* createWindow( int width, int height )
//-----------------------------------------------------------------------------
{
    MyWindow* pWindow = new MyWindow( width, height );
    if( pWindow->can_do() == 0 )
    {
        cout << "OpenGL not possible!!" << endl;
    }
    else
    {
        cout << "Using OpenGL." << endl;
        if( pWindow->can_do_overlay() == 0 )
        {
            cout << "OpenGL Overlays not possible!!" << endl;
        }
        else
        {
            cout << "Using OpenGL Overlays." << endl;
        }

        //window->clear_border();
        gl_font( FL_TIMES, 12 );
        Fl::gl_visual( FL_RGB );
        pWindow->end();
        pWindow->show();
    }
    return pWindow;
}
#else
//-----------------------------------------------------------------------------
class MyWindow : public Fl_Overlay_Window
    //-----------------------------------------------------------------------------
{
    const Request* pRequest_;
    string msg_;
    virtual void draw()
    {
        if( pRequest_ )
        {
            fl_draw_image( ( unsigned char* ) pRequest_->imageData.read(), 0, 0,
                           pRequest_->imageWidth.read(), pRequest_->imageHeight.read(),
                           pRequest_->imageBytesPerPixel.read(),
                           pRequest_->imageLinePitch.read() );
        }
    }
    virtual void draw_overlay()
    {
        fl_color( FL_RED );
        fl_draw( msg_.c_str(), 8, h() - 20 );
    }
public:
    MyWindow( int W, int H ) : Fl_Overlay_Window( W, H ), pRequest_( 0 ), msg_( "" ) {}
    void NewRequest( const Request* pRequest )
    {
        pRequest_ = pRequest;
    }
    void setOverlayString( const string& msg )
    {
        msg_ = msg;
        redraw_overlay();
    }
};

//-----------------------------------------------------------------------------
MyWindow* createWindow( int width, int height )
//-----------------------------------------------------------------------------
{
    cout << "Not using OpenGL." << endl;
    MyWindow* pWindow = new MyWindow( width, height );
    //pWindow->clear_border();
    fl_font( FL_TIMES, 12 );
    pWindow->end();
    Fl::visual( FL_RGB | FL_DOUBLE | FL_INDEX );
    return pWindow;
}

#endif

//-----------------------------------------------------------------------------
void windowCallback( Fl_Widget*, void* )
//-----------------------------------------------------------------------------
{
    printf( "Window was closed\n" );
    g_boTerminated = true;
}

//-----------------------------------------------------------------------------
struct DisplayInfo
//-----------------------------------------------------------------------------
{
    MyWindow* pDisp;
    // we always have to keep one frame locked as the display module might want to repaint the image e.g. during sizing, thus we
    // can free it unless we have a assigned the display to a new buffer.
    int lastRequestNr;
    unsigned int framesCaptured;
    explicit DisplayInfo( MyWindow* p ) : pDisp( p ), lastRequestNr( INVALID_ID ), framesCaptured( 0 ) {}
};

//-----------------------------------------------------------------------------
struct ThreadParameter
//-----------------------------------------------------------------------------
{
    Device*              pDev;
    vector<DisplayInfo*> displayData;
    const int            SENSOR_HEAD_COUNT;
    ThreadParameter( Device* p, const int shc ) : pDev( p ), SENSOR_HEAD_COUNT( shc ) {}
};

//-----------------------------------------------------------------------------
bool setupHRTC( Device* pDev, int frameRate_Hz, int exposureTime_us, int sensorHeadCount )
//-----------------------------------------------------------------------------
{
    cout << "Trying to capture at " << frameRate_Hz << " frames per second. Please make sure the device can deliver this frame rate." << endl;

    int frametime_us = static_cast<int>( 1000000.0 * ( 1.0 / static_cast<double>( frameRate_Hz ) ) );
    const int TRIGGER_PULSE_WIDTH_us = 100;
    if( frametime_us < 2 * TRIGGER_PULSE_WIDTH_us )
    {
        cout << "frame rate too high (" << frameRate_Hz << "). Will use 5 Hz." << endl;
        frametime_us = 200000;
    }

    if( exposureTime_us > ( frametime_us - 2 * TRIGGER_PULSE_WIDTH_us ) )
    {
        cout << "exposure time too high(" << exposureTime_us << "). Will use " << frametime_us - 2 * TRIGGER_PULSE_WIDTH_us << " instead" << endl;
        exposureTime_us = frametime_us - 2 * TRIGGER_PULSE_WIDTH_us;
    }

    {
        CameraSettingsBlueCOUGAR csbd( pDev );
        csbd.expose_us.write( exposureTime_us );
        // define a HRTC program that results in a define image frequency
        // the hardware real time controller shall be used to trigger an image
        csbd.triggerSource.write( ctsRTCtrl );
        csbd.triggerMode.write( ctmOnFallingEdge );
    }

    IOSubSystemCommon iossc( pDev );
    // error checks
    if( iossc.RTCtrProgramCount() == 0 )
    {
        // no HRTC controllers available
        cout << "This device (" << pDev->product.read() << ") doesn't support HRTC" << endl;
        return false;
    }

    RTCtrProgram* pRTCtrlprogram = iossc.getRTCtrProgram( 0 );
    if( !pRTCtrlprogram )
    {
        // this only should happen if the system is short of memory
        cout << "Error! No valid program. Short of memory?" << endl;
        return false;
    }

    // start of the program
    // we need 5 steps for the program
    pRTCtrlprogram->setProgramSize( 5 );

    // wait a certain amount of time to achieve the desired frequency
    int progStep = 0;
    int i = 0;
    RTCtrProgramStep* pRTCtrlStep = 0;
    pRTCtrlStep = pRTCtrlprogram->programStep( progStep++ );
    pRTCtrlStep->opCode.write( rtctrlProgWaitClocks );
    pRTCtrlStep->clocks_us.write( frametime_us - exposureTime_us );

    // trigger both sensor heads
    pRTCtrlStep = pRTCtrlprogram->programStep( progStep++ );
    pRTCtrlStep->opCode.write( rtctrlProgTriggerSet );
    for( i = 0; i < sensorHeadCount; i++ )
    {
        pRTCtrlStep->sensorHeads.write( digioOn, i );
    }

    // high time for the trigger signal (should not be smaller than 100 us)
    pRTCtrlStep = pRTCtrlprogram->programStep( progStep++ );
    pRTCtrlStep->opCode.write( rtctrlProgWaitClocks );
    pRTCtrlStep->clocks_us.write( exposureTime_us );

    // end trigger signal
    pRTCtrlStep = pRTCtrlprogram->programStep( progStep++ );
    pRTCtrlStep->opCode.write( rtctrlProgTriggerReset );
    for( i = 0; i < sensorHeadCount; i++ )
    {
        pRTCtrlStep->sensorHeads.write( digioOff, i );
    }

    // restart the program
    pRTCtrlStep = pRTCtrlprogram->programStep( progStep++ );
    pRTCtrlStep->opCode.write( rtctrlProgJumpLoc );
    pRTCtrlStep->address.write( 0 );

    // start the program
    pRTCtrlprogram->mode.write( rtctrlModeRun );

    // Now this device will deliver synchronous images at exactly the desired frequency
    // when it is constantly feed with image requests and the device can deliver
    // images at this frequency.

    return true;
}

//-----------------------------------------------------------------------------
unsigned int liveLoop( void* pData )
//-----------------------------------------------------------------------------
{
    ThreadParameter* pThreadParam = reinterpret_cast<ThreadParameter*>( pData );
    unsigned int cnt = 0;

    if( !pThreadParam->pDev->isOpen() )
    {
        cout << "Initialising the device. This might take some time..." << endl;
        try
        {
            pThreadParam->pDev->open();
        }
        catch( const ImpactAcquireException& e )
        {
            // this e.g. might happen if the same device is already opened in another process...
            cout << "An error occurred while opening device " << pThreadParam->pDev->serial.read()
                 << "(error code: " << e.getErrorCodeAsString() << "). Press [ENTER] to end the application..." << endl;
            cin.get();
            return 0;
        }
    }

    vector<DisplayInfo*>::size_type displayCount = pThreadParam->displayData.size();

    // establish access to the statistic properties
    Statistics statistics( pThreadParam->pDev );
    // create an interface to the device found
    FunctionInterface fi( pThreadParam->pDev );

    // make sure enough requests are available
    SystemSettings ss( pThreadParam->pDev );
    ss.requestCount.write( static_cast<int>( pThreadParam->displayData.size()*fi.requestCount() ) );
    // pre-fill the capture queue. There can be more than 1 queue for some devices, but for this sample
    // we will work with the default capture queue. If a device supports more than one capture or result
    // queue, this will be stated in the manual. If nothing is mentioned about it, the device supports one
    // queue only.

    int input = 0;
    Connector connector( pThreadParam->pDev );

    // prepare all sensor heads
    ImageRequestControl irc( pThreadParam->pDev );
    irc.mode.write( ircmTrial );
    for( int i = 0; i < pThreadParam->SENSOR_HEAD_COUNT; i++ )
    {
        connector.videoChannel.write( i );
        fi.imageRequestSingle( &irc );
        int trialRequestNr = fi.imageRequestWaitFor( -1 );
        if( !fi.isRequestNrValid( trialRequestNr ) )
        {
            cout << "An error occurred while preparing device " << pThreadParam->pDev->serial.read()
                 << "(error code: " << ImpactAcquireException::getErrorCodeAsString( trialRequestNr ) << "). Press [ENTER] to end the application..." << endl;
            cin.get();
            return 0;
        }
        fi.imageRequestUnlock( trialRequestNr );
    }

    // now all the devices sensor heads have been setup to use the current settings.
    // We can now request 'real' frames
    irc.mode.write( ircmManual );
    do
    {
        connector.videoChannel.write( input );
        input = ( input + 1 ) % pThreadParam->SENSOR_HEAD_COUNT;
    }
    while( fi.imageRequestSingle( &irc ) == DMR_NO_ERROR );

    // run thread loop
    const Request* pRequest = 0;
    const unsigned int timeout_ms = 2500;
    int requestNr = INVALID_ID;

    while( !g_boTerminated )
    {
        // wait for results from the default capture queue
        requestNr = fi.imageRequestWaitFor( timeout_ms );
        if( fi.isRequestNrValid( requestNr ) )
        {
            pRequest = fi.getRequest( requestNr );
            int videoChannel = pRequest->infoVideoChannel.read();
            if( pRequest->isOK() )
            {
                ++pThreadParam->displayData.at( videoChannel )->framesCaptured;
                pThreadParam->displayData[videoChannel]->pDisp->NewRequest( pRequest );
                pThreadParam->displayData[videoChannel]->pDisp->redraw();
                Fl::check();
                if( fi.isRequestNrValid( pThreadParam->displayData[videoChannel]->lastRequestNr ) )
                {
                    // this image is no longer needed by the display...
                    fi.imageRequestUnlock( pThreadParam->displayData[videoChannel]->lastRequestNr );
                }
                pThreadParam->displayData[videoChannel]->lastRequestNr = requestNr;
            }
            else
            {
                cout << "Error: " << pRequest->requestResult.readS() << endl;
                fi.imageRequestUnlock( requestNr );
            }

            // send a new image request into the capture queue
            connector.videoChannel.write( videoChannel );
            fi.imageRequestSingle( &irc );

            ++cnt;
            // here we can display some statistical information every 100th image
            if( cnt % 100 == 0 )
            {
                cout << "Info from " << pThreadParam->pDev->serial.read()
                     << ": " << statistics.errorCount.name() << ": " << statistics.errorCount.readS()
                     << ", " << statistics.framesIncompleteCount.name() << ": " << statistics.framesIncompleteCount.readS()
                     << ", " << statistics.missingDataAverage_pc.name() << ": " << statistics.missingDataAverage_pc.readS();
                for( vector<DisplayInfo*>::size_type i = 0; i < displayCount; i++ )
                {
                    cout << ", channel[" << i << "]: " << pThreadParam->displayData[i]->framesCaptured << " frames captured";
                    ostringstream oss;
                    oss << "channel[" << i << "]: " << pThreadParam->displayData[i]->framesCaptured << " frames captured";
                    pThreadParam->displayData[i]->pDisp->setOverlayString( oss.str() );
                }
                cout << endl;
            }
        }
        else
        {
            // If the error code is -2119(DEV_WAIT_FOR_REQUEST_FAILED), the documentation will provide
            // additional information under TDMR_ERROR in the interface reference
            cout << "imageRequestWaitFor failed (" << requestNr << ", " << ImpactAcquireException::getErrorCodeAsString( requestNr ) << ")"
                 << ", timeout value too small?" << endl;
        }
    }

    for( vector<DisplayInfo*>::size_type i = 0; i < displayCount; i++ )
    {
        // stop the display from showing freed memory
        pThreadParam->displayData[i]->pDisp->NewRequest( 0 );
        // free buffer that is locked for the display
        if( fi.isRequestNrValid( pThreadParam->displayData[i]->lastRequestNr ) )
        {
            fi.imageRequestUnlock( pThreadParam->displayData[i]->lastRequestNr );
        }
    }

    // free the buffer that might just have been ready
    if( fi.isRequestNrValid( requestNr ) )
    {
        fi.imageRequestUnlock( requestNr );
    }

    // clear the request queue
    fi.imageRequestReset( 0, 0 );
    return 0;
}

//-----------------------------------------------------------------------------
bool isDeviceSupportedBySample( const Device* const pDev )
//-----------------------------------------------------------------------------
{
    return ( match( pDev->product.read(), string( "mvBlueLYNX-M7" ), '*' ) == 0 );
}

//-----------------------------------------------------------------------------
int main( int argc, char* argv[] )
//-----------------------------------------------------------------------------
{
    DeviceManager devMgr;
    cout << "This sample is meant for mvBlueLYNX-M7 devices only. Other devices might be installed" << endl
         << "but won't be recognized by the application." << endl
         << endl;

    int exposureTime_us = 20000;
    int frameRate_Hz = 5;
    int width = -1;
    int height = -1;
    int interPacketDelay = 25;

    // scan command line
    if( argc > 1 )
    {
        for( int i = 1; i < argc; i++ )
        {
            string param( argv[i] ), key, value;
            string::size_type keyEnd = param.find_first_of( "=" );
            if( ( keyEnd == string::npos ) || ( keyEnd == param.length() - 1 ) )
            {
                cout << "Invalid command line parameter: '" << param << "' (ignored)." << endl;
            }
            else
            {
                key = param.substr( 0, keyEnd );
                value = param.substr( keyEnd + 1 );
                if( ( key == "exposureTime" ) || ( key == "et" ) )
                {
                    exposureTime_us = static_cast<int>( atoi( value.c_str() ) );
                }
                else if( ( key == "frameRate" ) || ( key == "fr" ) )
                {
                    frameRate_Hz = static_cast<int>( atoi( value.c_str() ) );
                }
                else if( ( key == "width" ) || ( key == "w" ) )
                {
                    width = static_cast<int>( atoi( value.c_str() ) );
                }
                else if( ( key == "height" ) || ( key == "h" ) )
                {
                    height = static_cast<int>( atoi( value.c_str() ) );
                }
                else if( key == "scpd" )
                {
                    interPacketDelay = static_cast<int>( atoi( value.c_str() ) );
                }
                else
                {
                    cout << "Invalid command line parameter: '" << param << "' (ignored)." << endl;
                }
            }
        }
    }
    else
    {
        cout << "No command line parameters specified. Available parameters:" << endl
             << "  'frameRate' or 'fr' to specify the frame rate(frames per second per sensor head) of the resulting data stream" << endl
             << "  'exposureTime' or 'et' to specify the exposure time per frame in us" << endl
             << "  'width' or 'w' to specify the width of the AOI" << endl
             << "  'height' or 'h' to specify the height of the AOI" << endl
             << endl
             << "USAGE EXAMPLE:" << endl
             << "  SynchronousCaptureMultipleInputs et=5000 frameRate=5" << endl << endl;
    }

    Device* pDev = getDeviceFromUserInput( devMgr, isDeviceSupportedBySample );
    if( !pDev )
    {
        cout << "Unable to continue!";
        cout << "Press [ENTER] to end the application" << endl;
        cin.get();
        return 0;
    }

    try
    {
        cout << "Please note, that this sample (depending on the selected frame rate and resolution) might require a lot" << endl
             << "of network bandwidth, thus to achieve optimal results, it's crucial to have" << endl
             << "  - a good, reliable network controller (we recommend the Intel PRO/1000 series)" << endl
             << "  - the latest driver for the network controller installed" << endl
             << "  - jumbo frames enabled and the receive and transmit buffer for the network controller set to max. values" << endl
             << "  - the InterfaceMTU on the mvBlueLYNX-M7 set to its maximum value" << endl
             << endl
             << "In case of 'rrFrameIncomplete' errors the reason is most certainly to be found in one of"
             << "the requirements listed above not being met."
             << endl;

        cout << "Will try to synchronize sensor heads(" << frameRate_Hz << " fps per head with " << exposureTime_us << "us exposure time per frame) now" << endl
             << "During this operation the device will be initialised. This might take some time..." << endl;

        Connector connector( pDev );
        CameraSettingsBlueCOUGAR cs( pDev );
        try
        {
            if( width != -1 )
            {
                cs.aoiWidth.write( width );
            }
            if( height != -1 )
            {
                cs.aoiHeight.write( height );
            }
        }
        catch( const ImpactAcquireException& e )
        {
            cout << "Failed to set up AOI: " << e.getErrorString() << "(" << e.getErrorCodeAsString() << ")" << endl;
        }
        const int SENSOR_HEAD_COUNT = connector.videoChannel.read( plMaxValue ) + 1;
        setupHRTC( pDev, frameRate_Hz, exposureTime_us, SENSOR_HEAD_COUNT );

        // initialise display windows
        // IMPORTANT: It's NOT save to create multiple display windows in multiple threads!!!
        ThreadParameter threadParam( pDev, SENSOR_HEAD_COUNT );
        DeviceComponentLocator locator( pDev, dltSystemSettings );
        PropertyI64 gevStreamChannelSelector;
        locator.bindComponent( gevStreamChannelSelector, "GevStreamChannelSelector" );
        PropertyI64 gevSCPD;
        locator.bindComponent( gevSCPD, "GevSCPD" );
        for( int i = 0; i < SENSOR_HEAD_COUNT; i++ )
        {
            gevStreamChannelSelector.write( i );
            gevSCPD.write( interPacketDelay );
            MyWindow* p = createWindow( cs.aoiWidth.read(), cs.aoiHeight.read() );
            p->show();
            p->callback( windowCallback );
            threadParam.displayData.push_back( new DisplayInfo( p ) );
        }

        // start the execution of the 'live' thread.
        cout << "Close any of the display windows to end the application" << endl;
        liveLoop( &threadParam );
        vector<DisplayInfo*>::size_type displayCount = threadParam.displayData.size();
        for( vector<DisplayInfo*>::size_type i = 0; i < displayCount; i++ )
        {
            delete threadParam.displayData[i]->pDisp;
            delete threadParam.displayData[i];
            threadParam.displayData[i] = 0;
        }
    }
    catch( const ImpactAcquireException& e )
    {
        // this e.g. might happen if the same device is already opened in another process...
        cout << "An error occurred while configuring device " << pDev->serial.read()
             << "(error code: " << e.getErrorCodeAsString() << "). Press [ENTER] to end the application..." << endl;
        cin.get();
    }
    return 0;
}

