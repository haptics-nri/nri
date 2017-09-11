//-----------------------------------------------------------------------------
#ifndef DeviceHandlerBlueDeviceH
#define DeviceHandlerBlueDeviceH DeviceHandlerBlueDeviceH
//-----------------------------------------------------------------------------
#include <common/auto_array_ptr.h>
#include "DeviceHandler.h"
#include "PackageDescriptionParser.h"

//-----------------------------------------------------------------------------
class DeviceHandlerBlueDevice : public DeviceHandler
//-----------------------------------------------------------------------------
{
    enum TProductGroup
    {
        pgUnknown,
        pgBlueCOUGAR_P,
        pgBlueCOUGAR_S,
        pgBlueCOUGAR_X,
        pgBlueFOX3,
        pgBlueLYNX_M7
    };
    TProductGroup product_;
    wxString productStringForFirmwareUpdateCheck_;
    wxString firmwareUpdateFileName_;
    wxString firmwareUpdateFolder_;
    wxString firmwareUpdateDefaultFolder_;
    wxString firmwareUpdateFolderDevelopment_;
    wxString GenICamFile_;
    wxString temporaryFolder_;
    std::vector<std::string> userSetsToKeepDuringUpdate_;
    int CheckForIncompatibleFirmwareVersions_BlueCOUGAR_X( bool boSilentMode, const wxString& serial, const FileEntryContainer& fileEntries, const wxString& selection, const Version& currentFirmwareVersion );
    TUpdateResult DoFirmwareUpdate_BlueCOUGAR_X( bool boSilentMode, const wxString& serial, const char* pBuf, const size_t bufSize );
    TUpdateResult DoFirmwareUpdate_BlueFOX3( bool boSilentMode, const wxString& serial, const char* pBuf, const size_t bufSize );
    bool ExtractFileVersion( const wxString& fileName, Version& fileVersion ) const;
    static bool GetFileFromArchive( const wxString& firmwareFileAndPath, const char* pArchive, size_t archiveSize, const wxString& filename, auto_array_ptr<char>& data, DeviceConfigureFrame* pParent );
    int GetLatestFirmwareVersionCOUGAR_XAndFOX3Device( Version& latestFirmwareVersion ) const;
    bool IsBlueCOUGAR_X( void ) const;
    bool IsBlueFOX3( void ) const;
    bool IsFirmwareUpdateMeaningless( bool boSilentMode, const Version& deviceFWVersion, const Version& selectedFWVersion, const wxString& defaultFWArchive, const Version& defaultFolderFWVersion ) const;
    static TUpdateResult ParseUpdatePackageCOUGAR_XAndFOX3Device( PackageDescriptionFileParser& fileParser, const wxString& firmwareFileAndPath, DeviceConfigureFrame* pParent, auto_array_ptr<char>& pBuffer );
    void SelectCustomGenICamFile( const wxString& descriptionFile = wxEmptyString );
    int UpdateCOUGAR_SDevice( bool boSilentMode );
    int UpdateCOUGAR_XAndFOX3Device( bool boSilentMode, bool boPersistentUserSets );
    int UpdateLYNX_M7AndCOUGAR_PDevice( const wxString& updateFileName, const wxString& fileExtension, bool boSilentMode );
    int UploadFile( const wxString& fullPath, const wxString& descriptionFile );
    void UserSetBackup( void );
    void UserSetRestore( const wxString& previousUserSetDefaultValueToRestore );
    wxString SetUserSetDefault( const wxString& userSetDefaultValue );
public:
    DeviceHandlerBlueDevice( mvIMPACT::acquire::Device* pDev );
    static DeviceHandler* Create( mvIMPACT::acquire::Device* pDev )
    {
        return new DeviceHandlerBlueDevice( pDev );
    }
    virtual bool GetIDFromUser( long& newID, const long minValue, const long maxValue );
    virtual int GetLatestFirmwareVersion( Version& latestFirmwareVersion ) const;
    virtual bool SupportsFirmwareUpdate( void ) const;
    virtual int UpdateFirmware( bool boSilentMode, bool boPersistentUserSets );
    virtual void SetCustomFirmwareFile( const wxString& customFirmwareFile );
    virtual void SetCustomFirmwarePath( const wxString& customFirmwarePath );
    virtual void SetCustomGenICamFile( const wxString& customGenICamFile );
};

int        CompareFileVersion( const wxString& first, const wxString& second );
wxString   ExtractVersionNumber( const wxString& s );
bool       GetNextNumber( wxString& str, long& number );
int64_type MACAddressFromString( const std::string& MAC );

#endif // DeviceHandlerBlueDeviceH
