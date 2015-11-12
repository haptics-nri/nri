//-----------------------------------------------------------------------------
#ifndef CaptureThreadH
#define CaptureThreadH CaptureThreadH
//-----------------------------------------------------------------------------
#include "DevData.h"
#include <deque>
#include <list>
#include <mvIMPACT_CPP/mvIMPACT_acquire.h>
#include <wx/thread.h>
#include <wx/string.h>

class wxEvtHandler;
class wxWindow;
struct RequestInfoData;

typedef std::deque<int> RequestContainer;

#define MULTI_SETTING_HACK 1

//-----------------------------------------------------------------------------
enum TEndOfSequenceReason
//-----------------------------------------------------------------------------
{
    eosrRecordingDone,
    eosrMultiFrameDone
};

//------------------------------------------------------------------------------
class CaptureThread : public wxThread
//------------------------------------------------------------------------------
{
    static unsigned int                         m_activeInstance;
    volatile bool                               m_boLive;
    volatile bool                               m_boRecord;
    volatile bool                               m_boContinuousRecording;
    volatile bool                               m_boForwardIncompleteFrames;
    mutable wxCriticalSection                   m_critSect;
    mutable wxCriticalSection                   m_critSectConfig;
    mutable int                                 m_currentRequest;
    int                                         m_discardedImages;
    static unsigned int                         m_instanceCnt;
    unsigned int                                m_instanceNr;
    wxString                                    m_lastErrorMessage;
    wxString                                    m_lastRequestResult;
    std::string                                 m_currentPlotInfoPath;
    mutable int                                 m_nextPendingRequest;           // for legacy mode used for a single display. DO NOT REMOVE UNTIL YOU ARE ABSOLUTELY SURE THAT THIS DOESN'T BREAK ANYTHING
    std::list<int>::size_type                   m_pendingRequestsForDisplayMax; // for new mode used with 2 or more displays
    mutable std::list<int>                      m_pendingRequestsForDisplay;    // for new mode used with 2 or more displays
    int                                         m_parentWindowID;
    mvIMPACT::acquire::Device*                  m_pDev;
    mvIMPACT::acquire::FunctionInterface*       m_pFuncInterface;
    mvIMPACT::acquire::ImageRequestControl      m_ImageRequestControl;
#if MULTI_SETTING_HACK
    int                                         m_SettingIndex;
    std::vector<std::pair<std::string, int> >   m_Settings;
    std::vector<unsigned int>                   m_PendingRequests;
    std::vector<int64_type>                     m_LastPendingRequestFrameNr;
    std::vector<unsigned int>                   m_RequestFailed;
    std::vector<unsigned int>::size_type        m_SettingCount;
    unsigned int                                m_CurrentSetting;
    TCaptureSettingUsageMode                    m_CaptureSettingUsageMode;
#endif // #if MULTI_SETTING_HACK
    wxEvtHandler*                               m_pParentWindowEventHandler;
    RequestContainer                            m_recordedSequence;
    size_t                                      m_currentSequenceIndexToDisplay;
    size_t                                      m_currentCountToRecord;
    size_t                                      m_totalCountToRecord;
    size_t                                      m_maxNumberToRequest;
    size_t                                      m_numberRequested;
    size_t                                      m_numberReturned;
    size_t                                      m_SkippedImageCount;
    int                                         m_captureQueueDepth;
    int                                         m_currentlyActiveRequests;

    bool                                        CheckSequenceRestart( bool boInformUserOnRecordStart ) const;
    void                                        CollectBufferInfo( ComponentIterator it, std::vector<wxString>& infoStrings, const wxString& path = wxT( "" ) ) const;
    int                                         FreeLastRequestFromSequence( int requestNotToUnlock );
    void                                        GetRequestInfoData( RequestInfoData* pReqInfo, const Request* pReq, HOBJ settingUsed ) const;
    bool                                        InternalSetLiveMode( bool boOn, bool boInformUserOnRecordStart = true );
    bool                                        InternalSetRecordMode( bool boOn, bool boUnlockCurrentRequest );
    int                                         RequestImages( int* pFramesRequested = 0 );
    void                                        UnlockPendingRequests( void );
protected:
    void*                                       Entry( void );
public:
    explicit                                    CaptureThread( mvIMPACT::acquire::FunctionInterface* pFuncInterface, mvIMPACT::acquire::Device* pDev, wxWindow* pParentWindow, unsigned int pendingImageQueueDepth );
    ~CaptureThread();
    size_t                                      DisplaySequenceRequest( size_t nr = 0 );
    size_t                                      DisplaySequenceRequestNext( void );
    size_t                                      DisplaySequenceRequestPrev( void );
    void                                        FreeSequence( int requestNotToUnlock = INVALID_ID );
    int                                         GetCaptureQueueDepth( void ) const
    {
        return m_captureQueueDepth;
    }
    bool                                        GetContinuousRecording( void ) const
    {
        return m_boContinuousRecording;
    }
    size_t                                      GetCurrentSequenceIndex( void ) const
    {
        return m_currentSequenceIndexToDisplay;
    }
    mvIMPACT::acquire::TRequestResult           GetImageData( RequestData& requestData ) const;
    void                                        GetImageInfo( bool boFillInfoVector, std::vector<wxString>& infoStrings ) const;
    wxString                                    GetLastRequestResult( void ) const;
    bool                                        GetLiveMode( void ) const
    {
        return m_boLive;
    }
    size_t                                      GetMultiFrameSequenceSize( void ) const
    {
        return m_maxNumberToRequest;
    }
    const RequestContainer&                     GetRecordedSequence( void ) const
    {
        return m_recordedSequence;
    }
    bool                                        GetRecordMode( void ) const
    {
        return m_boRecord;
    }
    size_t                                      GetRecordSequenceSize( void ) const
    {
        return m_totalCountToRecord;
    }
    size_t                                      GetSkippedImageCount( void );
    int                                         RecordSequence( void );
    int                                         RequestSingle( bool boInformUserOnRecordStart );
    void                                        SetActive( void );
    void                                        SetCaptureMode( bool boForwardIncompleteFrames );
    void                                        SetCaptureQueueDepth( int queueDepth );
#if MULTI_SETTING_HACK
    void                                        SetCaptureSettingUsageMode( TCaptureSettingUsageMode mode );
#endif // #if MULTI_SETTING_HACK
    void                                        SetContinuousRecording( bool boContinuous );
    void                                        SetCurrentPlotInfoPath( const std::string& currentPlotInfoPath );
    bool                                        SetLiveMode( bool boOn, bool boInformUserOnRecordStart = true );
    void                                        SetMultiFrameSequenceSize( size_t sequenceSize );
    bool                                        SetRecordMode( bool boOn );
    void                                        SetRecordSequenceSize( size_t sequenceSize );
    void                                        UnlockRequest( int nr, bool boForceUnlock = false );
#if MULTI_SETTING_HACK
    void                                        UpdateSettingTable( void );
    int                                         RequestReset( void );
#endif // #if MULTI_SETTING_HACK
};

#endif // CaptureThreadH
