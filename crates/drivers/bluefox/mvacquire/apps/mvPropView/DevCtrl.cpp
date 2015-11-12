//-----------------------------------------------------------------------------
#include <apps/Common/wxAbstraction.h>
#include "CaptureThread.h"
#include "DataConversion.h"
#include "DevCtrl.h"
#include <mvIMPACT_CPP/mvIMPACT_acquire_GenICam.h>
#include "PropTree.h"
#include <algorithm>
#include <functional>

using namespace std;
using namespace mvIMPACT::acquire;

//-----------------------------------------------------------------------------
class PropGridFrozenScope
//-----------------------------------------------------------------------------
{
    wxPropertyGrid* pPG_;
public:
    explicit PropGridFrozenScope( wxPropertyGrid* pPG ) : pPG_( pPG )
    {
        pPG_->Freeze();
    }
    ~PropGridFrozenScope()
    {
        pPG_->Thaw();
    }
};

//-----------------------------------------------------------------------------
DevicePropertyHandler::DevicePropertyHandler( wxPropertyGrid* pPGDriver, wxPropertyGrid* pPGDevice, bool boDisplayDebugInfo, bool boDisplayFullTree, bool boDisplayInvisibleComponents ) :
    m_actDevListChangedCounter( 0 ), m_actDrvListChangedCounter( 0 ), m_boDisplayDebugInfo( boDisplayDebugInfo ), m_boDisplayInvisibleComponents( boDisplayInvisibleComponents ),
    m_componentVisibility( cvBeginner ), m_fullTreeChangedCounter( 0 ), m_pFullTree( 0 ), m_pActiveDevData( 0 ), m_pActiveDevice( 0 ),
    m_pPGDevice( pPGDevice ), m_pPGDriver( pPGDriver ), m_viewMode( vmStandard ), m_boUseHexIndices( false ), m_boUseDisplayNames( false ), m_boUseSelectorGrouping( false )
//-----------------------------------------------------------------------------
{
    if( boDisplayFullTree )
    {
        m_pFullTree = new PropTree( ROOT_LIST, "Full Tree", m_pPGDevice, static_cast<EDisplayFlags>( GetDisplayFlags() ) );
        m_pFullTree->Draw( false );
    }
}

//-----------------------------------------------------------------------------
DevicePropertyHandler::~DevicePropertyHandler()
//-----------------------------------------------------------------------------
{
    PropGridFrozenScope PGDriverFrozenScope( m_pPGDriver );
    PropGridFrozenScope PGDeviceFrozenScope( m_pPGDevice );
    DevToTreeMap::iterator itStart = m_devToTreeMap.begin();
    DevToTreeMap::iterator itEnd = m_devToTreeMap.end();
    while( itStart != itEnd )
    {
        CloseDriver( ConvertedString( itStart->first->serial.read() ) );
        if( itStart->second->pCaptureThread )
        {
            itStart->second->pCaptureThread->Delete();
            delete itStart->second->pCaptureThread;
            itStart->second->pCaptureThread = 0;
        }
        DeleteDriverTrees( itStart );
        delete itStart->second->pDriverTree;
        delete itStart->second->pDeviceTree;
        delete itStart->second->pFuncInterface;
        delete itStart->second->pStatistics;
        delete itStart->second->pInfo;
        delete itStart->second;
        ++itStart;
    }
    delete m_pFullTree;
}

//-----------------------------------------------------------------------------
void DevicePropertyHandler::AddFeatureToSetIfValid( set<HOBJ>& s, Component& c )
//-----------------------------------------------------------------------------
{
    if( c.isValid() )
    {
        s.insert( c.hObj() );
    }
}

//-----------------------------------------------------------------------------
void DevicePropertyHandler::CheckForWizards( mvIMPACT::acquire::Device* pDev, DeviceData* pDevData )
//-----------------------------------------------------------------------------
{
    if( pDev->interfaceLayout.isValid() && ( pDev->interfaceLayout.read() == dilGenICam ) )
    {
        {
            // check for SFNC compliant File Access Control
            mvIMPACT::acquire::GenICam::FileAccessControl fac( pDev );
            if( fac.fileAccessBuffer.isValid() && fac.fileAccessLength.isValid() && fac.fileAccessOffset.isValid() &&
                fac.fileOpenMode.isValid() && fac.fileOperationExecute.isValid() && fac.fileOperationResult.isValid() &&
                fac.fileOperationSelector.isValid() && fac.fileOperationStatus.isValid() && fac.fileSelector.isValid() )
            {
                set<HOBJ> s;
                s.insert( fac.fileAccessBuffer.parent().hObj() );
                s.insert( fac.fileAccessBuffer.hObj() );
                s.insert( fac.fileAccessLength.hObj() );
                s.insert( fac.fileAccessOffset.hObj() );
                s.insert( fac.fileOpenMode.hObj() );
                s.insert( fac.fileOperationExecute.hObj() );
                s.insert( fac.fileOperationResult.hObj() );
                s.insert( fac.fileOperationSelector.hObj() );
                s.insert( fac.fileOperationStatus.hObj() );
                s.insert( fac.fileSelector.hObj() );
                s.insert( fac.fileSize.hObj() );
                pDevData->supportedWizards.insert( make_pair( wFileAccessControl, s ) );
            }
        }

        {
            // check for SFNC compliant LUT Control
            mvIMPACT::acquire::GenICam::LUTControl lc( pDev );
            if( lc.LUTEnable.isValid() && lc.LUTIndex.isValid() &&
                lc.LUTValue.isValid() )
            {
                set<HOBJ> s;
                s.insert( lc.LUTEnable.parent().hObj() );
                s.insert( lc.LUTEnable.hObj() );
                s.insert( lc.LUTIndex.hObj() );
                AddFeatureToSetIfValid( s, lc.LUTSelector );
                s.insert( lc.LUTValue.hObj() );
                pDevData->supportedWizards.insert( make_pair( wLUTControl, s ) );
            }
        }

        // check for 'mvLensControl category
        try
        {
            // creating this object will raise an exception if the category is not supported!
            mvIMPACT::acquire::GenICam::mvLensControl lc( pDev );
            if( lc.mvDriveSelector.isValid() )
            {
                set<HOBJ> s;
                s.insert( lc.mvDriveSelector.parent().hObj() );
                s.insert( lc.mvDriveSelector.hObj() );
                AddFeatureToSetIfValid( s, lc.mvDriveBackward );
                AddFeatureToSetIfValid( s, lc.mvDriveDuration );
                AddFeatureToSetIfValid( s, lc.mvDriveForward );
                AddFeatureToSetIfValid( s, lc.mvDriveLevel );
                AddFeatureToSetIfValid( s, lc.mvIrisMode );
                AddFeatureToSetIfValid( s, lc.mvIrisSignalLevelMax );
                AddFeatureToSetIfValid( s, lc.mvIrisSignalLevelMin );
                AddFeatureToSetIfValid( s, lc.mvIrisType );
                pDevData->supportedWizards.insert( make_pair( wLensControl, s ) );
            }
        }
        catch( const ImpactAcquireException& )
        {
            // creating this object in case the category is not supported will raise an exception. This is intended!!!
        }
    }

    // currently only MATRIX VISION BF, BF3 & BC-X(D) devices offer the quick setup wizard
    const string product( pDev->product.read() );
    if( ( product.find( "mvBlueFOX3" ) != string::npos ) || ( product.find( "mvBlueCOUGAR-X" ) != string::npos ) )
    {
        string manufacturer( pDev->manufacturer.read() );
        transform( manufacturer.begin(), manufacturer.end(), manufacturer.begin(), ptr_fun<int, int>( tolower ) );
        if( manufacturer.find( "matrix vision" ) != string::npos )
        {
            pDevData->supportedWizards.insert( make_pair( wQuickSetup, set<HOBJ>() ) );
        }
    }
    else if( product.find( "mvBlueFOX" ) != string::npos )
    {
        pDevData->supportedWizards.insert( make_pair( wQuickSetup, set<HOBJ>() ) );
    }

    // check for mvIMPACT Acquire compliant color twist filter
    mvIMPACT::acquire::ImageProcessing ip( pDev );
    if( ip.colorTwistInputCorrectionMatrixEnable.isValid() &&
        ip.colorTwistInputCorrectionMatrixMode.isValid() &&
        ip.colorTwistInputCorrectionMatrixRow0.isValid() &&
        ip.colorTwistInputCorrectionMatrixRow1.isValid() &&
        ip.colorTwistInputCorrectionMatrixRow2.isValid() &&
        ip.colorTwistEnable.isValid() && ip.colorTwistRow0.isValid() &&
        ip.colorTwistRow1.isValid() && ip.colorTwistRow2.isValid() &&
        ip.colorTwistOutputCorrectionMatrixEnable.isValid() &&
        ip.colorTwistOutputCorrectionMatrixMode.isValid() &&
        ip.colorTwistOutputCorrectionMatrixRow0.isValid() &&
        ip.colorTwistOutputCorrectionMatrixRow1.isValid() &&
        ip.colorTwistOutputCorrectionMatrixRow2.isValid() &&
        ip.colorTwistResultingMatrixRow0.isValid() &&
        ip.colorTwistResultingMatrixRow1.isValid() &&
        ip.colorTwistResultingMatrixRow2.isValid() )
    {
        set<HOBJ> s;
        s.insert( ip.colorTwistEnable.parent().hObj() );
        s.insert( ip.colorTwistInputCorrectionMatrixEnable.hObj() );
        s.insert( ip.colorTwistInputCorrectionMatrixMode.hObj() );
        s.insert( ip.colorTwistInputCorrectionMatrixRow0.hObj() );
        s.insert( ip.colorTwistInputCorrectionMatrixRow1.hObj() );
        s.insert( ip.colorTwistInputCorrectionMatrixRow2.hObj() );
        s.insert( ip.colorTwistEnable.hObj() );
        s.insert( ip.colorTwistRow0.hObj() );
        s.insert( ip.colorTwistRow1.hObj() );
        s.insert( ip.colorTwistRow2.hObj() );
        s.insert( ip.colorTwistOutputCorrectionMatrixEnable.hObj() );
        s.insert( ip.colorTwistOutputCorrectionMatrixMode.hObj() );
        s.insert( ip.colorTwistOutputCorrectionMatrixRow0.hObj() );
        s.insert( ip.colorTwistOutputCorrectionMatrixRow1.hObj() );
        s.insert( ip.colorTwistOutputCorrectionMatrixRow2.hObj() );
        s.insert( ip.colorTwistResultingMatrixRow0.hObj() );
        s.insert( ip.colorTwistResultingMatrixRow1.hObj() );
        s.insert( ip.colorTwistResultingMatrixRow2.hObj() );
        if( pDev->interfaceLayout.isValid() && ( pDev->interfaceLayout.read() == dilGenICam ) )
        {
            // check for SFNC compliant Color Transformation Control features
            mvIMPACT::acquire::GenICam::ColorTransformationControl ctc( pDev );
            if( ctc.colorTransformationEnable.isValid() && ctc.colorTransformationValue.isValid() &&
                ctc.colorTransformationValueSelector.isValid() )
            {
                s.insert( ctc.colorTransformationEnable.hObj() );
                s.insert( ctc.colorTransformationValue.hObj() );
                s.insert( ctc.colorTransformationValueSelector.hObj() );
            }
        }
        pDevData->supportedWizards.insert( make_pair( wColorCorrection, s ) );
    }
}

//-----------------------------------------------------------------------------
void DevicePropertyHandler::CloseDriver( const wxString& name )
//-----------------------------------------------------------------------------
{
    string serialANSI( name.mb_str() );
    Device* pDev = m_devMgr.getDeviceBySerial( serialANSI );
    bool boMustRecreateFullTree = false;
    if( pDev && pDev->isOpen() )
    {
        DevToTreeMap::iterator it = m_devToTreeMap.find( pDev );
        if( it != m_devToTreeMap.end() )
        {
            wxCriticalSectionLocker locker( m_critSect );
            it->second->pCaptureThread->SetLiveMode( false );
            if( it->second->pCaptureThread )
            {
                it->second->pCaptureThread->Delete();
                delete it->second->pCaptureThread;
                it->second->pCaptureThread = 0;
            }
            it->second->boWasLive = false;
            {
                PropGridFrozenScope PGDriverFrozenScope( m_pPGDriver );
                PropGridFrozenScope PGDeviceFrozenScope( m_pPGDevice );
                if( m_pFullTree )
                {
                    delete m_pFullTree;
                    m_pFullTree = 0;
                    boMustRecreateFullTree = true;
                }
                else
                {
                    DeleteDriverTrees( it );
                }
            }
            delete it->second->pFuncInterface;
            it->second->pFuncInterface = 0;
            delete it->second->pStatistics;
            it->second->pStatistics = 0;
            delete it->second->pInfo;
            it->second->pInfo = 0;
            // we do not have an asignment operator
            ComponentLocator locator( pDev->hDev() );
            locator.bindComponent( it->second->acquisitionMode, "DUMMY_NON_FEATURE", 0, 1 );
            locator.bindComponent( it->second->acquisitionFrameCount, "DUMMY_NON_FEATURE", 0, 1 );
            it->second->supportedWizards.clear();
        }
        pDev->close();
    }

    if( boMustRecreateFullTree )
    {
        ReCreateFullTree();
    }
}

//-----------------------------------------------------------------------------
void DevicePropertyHandler::CreateDriverTrees( DevToTreeMap::iterator& it )
//-----------------------------------------------------------------------------
{
    if( !it->first->isOpen() )
    {
        return;
    }

    if( ( m_viewMode == vmStandard ) && it->second->pInfo->recommendedListsForUIs.isValid() )
    {
        m_vTreeProps.clear();
        const unsigned int cnt = it->second->pInfo->recommendedListsForUIs.valCount();
        for( unsigned int i = 0; i < cnt; i++ )
        {
            m_vTreeProps.push_back( TreeProp( it->second->pInfo->recommendedListsForUIs.read( i ), "" ) );
        }
#ifdef BUILD_WITH_PLUGIN_SUPPORT
        m_vTreeProps.push_back( TreeProp( "Plugins", "Plugins" ) );
#endif // #ifdef BUILD_WITH_PLUGIN_SUPPORT
    }
    else
    {
        PrepareTreeArray();
    }

    ComponentLocator cl( it->first->hDrv() );
    DeviceComponentLocator dcl( it->first->hDrv() );
    for( unsigned int i = 0; i < m_vTreeProps.size(); i++ )
    {
        try
        {
            PropTree* pTree = 0;
#ifdef BUILD_WITH_PLUGIN_SUPPORT
            if( m_vTreeProps[i].ListName == "Plugins" )
            {
                ComponentLocator tmpCL( 0 );
                pTree = new PropTree( tmpCL.findComponent( m_vTreeProps[i].ListName.c_str(), smIgnoreMethods | smIgnoreProperties ), m_vTreeProps[i].DisplayName.c_str(), m_pPGDevice, static_cast<EDisplayFlags>( GetDisplayFlags() ) );
            }
            else
#endif // #ifdef BUILD_WITH_PLUGIN_SUPPORT
                if( m_vTreeProps[i].ListType == dltUndefined )
                {
                    Component comp( cl.findComponent( m_vTreeProps[i].ListName.c_str(), smIgnoreMethods | smIgnoreProperties ) );
                    string displayName( comp.displayName() );
                    if( displayName.empty() )
                    {
                        displayName = m_vTreeProps[i].DisplayName.empty() ? comp.name() : m_vTreeProps[i].DisplayName;
                    }
                    pTree = new PropTree( comp.hObj(), displayName.c_str(), m_pPGDevice, static_cast<EDisplayFlags>( GetDisplayFlags() ) );
                }
                else
                {
                    pTree = new PropTree( dcl.bindSearchBaseList( it->first, m_vTreeProps[i].ListType ), m_vTreeProps[i].DisplayName.c_str(), m_pPGDevice, static_cast<EDisplayFlags>( GetDisplayFlags() ) );
                }
            it->second->vDriverTrees.push_back( pTree );
            pTree->Draw( false );
        }
        catch( const ImpactAcquireException& )
        {
            // this list is not supported by this device
        }
    }
}

//-----------------------------------------------------------------------------
void DevicePropertyHandler::DeleteDriverTrees( DevToTreeMap::iterator& it )
//-----------------------------------------------------------------------------
{
    while( !it->second->vDriverTrees.empty() )
    {
        delete it->second->vDriverTrees.back();
        it->second->vDriverTrees.pop_back();
    }
}

//-----------------------------------------------------------------------------
bool DevicePropertyHandler::DoesActiveDeviceSupportWizard( TWizardIDs wID ) const
//-----------------------------------------------------------------------------
{
    wxCriticalSectionLocker locker( m_critSect );
    if( m_pActiveDevData == 0 )
    {
        return false;
    }
    return m_pActiveDevData->supportedWizards.find( wID ) != m_pActiveDevData->supportedWizards.end();
}

//-----------------------------------------------------------------------------
EDisplayFlags DevicePropertyHandler::GetDisplayFlags( void ) const
//-----------------------------------------------------------------------------
{
    EDisplayFlags flags = dfNone;
    if( m_boDisplayDebugInfo )
    {
        flags = EDisplayFlags( flags | dfDisplayDebugInfo );
    }
    if( m_boDisplayInvisibleComponents )
    {
        flags = EDisplayFlags( flags | dfDisplayInvisibleComponents );
    }
    if( m_boUseHexIndices )
    {
        flags = EDisplayFlags( flags | dfHexIndices );
    }
    if( m_boUseDisplayNames )
    {
        flags = EDisplayFlags( flags | dfDisplayNames );
    }
    if( m_boUseSelectorGrouping )
    {
        flags = EDisplayFlags( flags | dfSelectorGrouping );
    }
    if( m_viewMode == vmDeveloper )
    {
        flags = EDisplayFlags( flags | dfDontUseFriendlyNamesForMethods );
    }
    return flags;
}

//-----------------------------------------------------------------------------
void DevicePropertyHandler::GetInterfaceClasses( const DeviceData* p, FunctionInterface** ppFI /* = 0 */, const Statistics** ppS /* = 0 */, CaptureThread** ppCT /* = 0 */ ) const
//-----------------------------------------------------------------------------
{
    if( p )
    {
        if( ppFI )
        {
            *ppFI = p->pFuncInterface;
        }
        if( ppS )
        {
            *ppS = p->pStatistics;
        }
        if( ppCT )
        {
            *ppCT = p->pCaptureThread;
        }
    }
}

//-----------------------------------------------------------------------------
Device* DevicePropertyHandler::GetActiveDevice( FunctionInterface** ppFI /* = 0 */, const Statistics** ppS /* = 0 */, CaptureThread** ppCT /* = 0 */ ) const
//-----------------------------------------------------------------------------
{
    wxCriticalSectionLocker locker( m_critSect );
    GetInterfaceClasses( m_pActiveDevData, ppFI, ppS, ppCT );
    return m_pActiveDevice;
}

//-----------------------------------------------------------------------------
mvIMPACT::acquire::Property DevicePropertyHandler::GetActiveDeviceAcquisitionMode( void ) const
//-----------------------------------------------------------------------------
{
    wxCriticalSectionLocker locker( m_critSect );
    return m_pActiveDevData ? m_pActiveDevData->acquisitionMode : Property();
}

//-----------------------------------------------------------------------------
mvIMPACT::acquire::PropertyI64 DevicePropertyHandler::GetActiveDeviceAcquisitionFrameCount( void ) const
//-----------------------------------------------------------------------------
{
    wxCriticalSectionLocker locker( m_critSect );
    return m_pActiveDevData ? m_pActiveDevData->acquisitionFrameCount : PropertyI64();
}

//-----------------------------------------------------------------------------
const WizardFeatureMap DevicePropertyHandler::GetActiveDeviceSupportedWizards( void ) const
//-----------------------------------------------------------------------------
{
    wxCriticalSectionLocker locker( m_critSect );
    return m_pActiveDevData ? m_pActiveDevData->supportedWizards : WizardFeatureMap();
}

//-----------------------------------------------------------------------------
bool DevicePropertyHandler::GetDeviceData( mvIMPACT::acquire::Device* pDev, mvIMPACT::acquire::FunctionInterface** ppFI /* = 0 */, const mvIMPACT::acquire::Statistics** ppS /* = 0 */, CaptureThread** ppCT /* = 0  */ ) const
//-----------------------------------------------------------------------------
{
    wxCriticalSectionLocker locker( m_critSect );
    DevToTreeMap::const_iterator it = m_devToTreeMap.find( pDev );
    if( it != m_devToTreeMap.end() )
    {
        GetInterfaceClasses( it->second, ppFI, ppS, ppCT );
        return true;
    }
    return false;
}

//-----------------------------------------------------------------------------
void DevicePropertyHandler::OpenDriver( const wxString& name, wxWindow* pParentWindow, unsigned int pendingImageQueueDepth )
//-----------------------------------------------------------------------------
{
    string serialANSI( name.mb_str() );
    Device* pDev = m_devMgr.getDeviceBySerial( serialANSI );
    if( pDev && !pDev->isOpen() )
    {
        UpdateTreeMap();
        pDev->open();
        DevToTreeMap::iterator it = m_devToTreeMap.find( pDev );
        if( it != m_devToTreeMap.end() )
        {
            {
                wxCriticalSectionLocker locker( m_critSect );
                if( !it->second->pFuncInterface )
                {
                    it->second->pFuncInterface = new FunctionInterface( pDev );
                }
                if( !it->second->pCaptureThread )
                {
                    it->second->pCaptureThread = new CaptureThread( it->second->pFuncInterface, pDev, pParentWindow, pendingImageQueueDepth );
                    it->second->pCaptureThread->Create();
                    it->second->pCaptureThread->SetPriority( 80 );
                    it->second->pCaptureThread->Run();
                    it->second->pCaptureThread->SetActive();
                }
                if( !it->second->pStatistics )
                {
                    it->second->pStatistics = new Statistics( pDev );
                }
                if( !it->second->pInfo )
                {
                    it->second->pInfo = new Info( pDev );
                }
                if( !it->second->acquisitionMode.isValid() )
                {
                    ComponentLocator locator( pDev->hDrv() );
                    locator.bindComponent( it->second->acquisitionMode, "AcquisitionMode" );
                }
                if( !it->second->acquisitionFrameCount.isValid() )
                {
                    ComponentLocator locator( pDev->hDrv() );
                    locator.bindComponent( it->second->acquisitionFrameCount, "AcquisitionFrameCount" );
                }
                CheckForWizards( pDev, it->second );
                if( !m_pFullTree &&
                    ( m_vTreeProps.empty() || ( it->second->vDriverTrees.size() != m_vTreeProps.size() ) ) )
                {
                    CreateDriverTrees( it );
                }
                it->second->boWasLive = false;
            }

            if( m_pFullTree )
            {
                m_pFullTree->Draw( false );
            }
        }
    }
}

//-----------------------------------------------------------------------------
void DevicePropertyHandler::PrepareTreeArray( void )
//-----------------------------------------------------------------------------
{
    m_vTreeProps.clear();
    switch( m_viewMode )
    {
    case vmDeveloper:
        m_vTreeProps.push_back( TreeProp( dltCameraDescriptions, "dmltCameraDescriptions" ) );
        m_vTreeProps.push_back( TreeProp( dltDeviceSpecificData, "dmltDeviceSpecificData" ) );
        m_vTreeProps.push_back( TreeProp( dltEventSubSystemSettings, "dmltEventSubSystemSettings" ) );
        m_vTreeProps.push_back( TreeProp( dltEventSubSystemResults, "dmltEventSubSystemResults" ) );
        m_vTreeProps.push_back( TreeProp( dltRequestCtrl, "dmltRequestCtrl" ) );
        m_vTreeProps.push_back( TreeProp( dltInfo, "dmltInfo" ) );
        m_vTreeProps.push_back( TreeProp( dltRequest, "dmltRequest" ) );
        m_vTreeProps.push_back( TreeProp( dltIOSubSystem, "dmltIOSubSystem" ) );
        m_vTreeProps.push_back( TreeProp( dltSetting, "dmltSetting" ) );
        m_vTreeProps.push_back( TreeProp( dltStatistics, "dmltStatistics" ) );
        m_vTreeProps.push_back( TreeProp( dltSystemSettings, "dmltSystemSettings" ) );
        m_vTreeProps.push_back( TreeProp( dltImageMemoryManager, "dmltImageMemoryManager" ) );
        break;
    case vmStandard:
    default:
        m_vTreeProps.push_back( TreeProp( dltRequestCtrl, "Request Controls" ) );
        m_vTreeProps.push_back( TreeProp( dltSetting, "Image Settings" ) );
        PrepareTreeArrayCommon();
        break;
    }
}

//-----------------------------------------------------------------------------
void DevicePropertyHandler::PrepareTreeArrayCommon( void )
//-----------------------------------------------------------------------------
{
    m_vTreeProps.push_back( TreeProp( dltCameraDescriptions, "Camera Descriptions" ) );
    m_vTreeProps.push_back( TreeProp( "EventSubSystem", "Events" ) );
    m_vTreeProps.push_back( TreeProp( dltIOSubSystem, "Digital I/O" ) );
    m_vTreeProps.push_back( TreeProp( dltImageMemoryManager, "Image Memory Manager" ) );
    m_vTreeProps.push_back( TreeProp( dltInfo, "Info" ) );
    m_vTreeProps.push_back( TreeProp( dltRequest, "Requests" ) );
    m_vTreeProps.push_back( TreeProp( dltStatistics, "Statistics" ) );
    m_vTreeProps.push_back( TreeProp( dltSystemSettings, "System Settings" ) );
    m_vTreeProps.push_back( TreeProp( dltDeviceSpecificData, "Device Specific Data" ) );
#ifdef BUILD_WITH_PLUGIN_SUPPORT
    m_vTreeProps.push_back( TreeProp( "Plugins", "Plugins" ) );
#endif // #ifdef BUILD_WITH_PLUGIN_SUPPORT
}

//-----------------------------------------------------------------------------
void DevicePropertyHandler::SetActiveDevice( const wxString& devName )
//-----------------------------------------------------------------------------
{
    wxCriticalSectionLocker locker( m_critSect );
    string serialANSI( devName.mb_str() );
    Device* pDev = m_devMgr.getDeviceBySerial( serialANSI );

    if( pDev )
    {
        UpdateTreeMap();
        DevToTreeMap::iterator itNewDev = m_devToTreeMap.find( pDev );

        if( itNewDev != m_devToTreeMap.end() )
        {
            if( m_pActiveDevice != pDev )
            {
                if( m_pActiveDevice )
                {
                    DevToTreeMap::iterator itCurDev = m_devToTreeMap.find( m_pActiveDevice );
                    if( itCurDev != m_devToTreeMap.end() )
                    {
                        if( !m_pFullTree )
                        {
                            {
                                PropGridFrozenScope PGDriverFrozenScope( m_pPGDriver );
                                delete itCurDev->second->pDriverTree;
                                itCurDev->second->pDriverTree = 0;
                            }
                            {
                                PropGridFrozenScope PGDeviceFrozenScope( m_pPGDevice );
                                delete itCurDev->second->pDeviceTree;
                                itCurDev->second->pDeviceTree = 0;
                                DeleteDriverTrees( itCurDev );
                            }
                        }
                        CaptureThread* pThread = itCurDev->second->pCaptureThread;
                        if( pThread )
                        {
                            itCurDev->second->boWasLive = pThread->GetLiveMode();
                            pThread->SetLiveMode( false );
                        }
                        else
                        {
                            itCurDev->second->boWasLive = false;
                        }
                    }
                }

                m_pActiveDevice = pDev;
                m_pActiveDevData = itNewDev->second;

                if( m_pFullTree )
                {
                    m_fullTreeChangedCounter = m_pFullTree->Draw( false );
                }
                else
                {
                    if( !itNewDev->second->pDriverTree )
                    {
                        itNewDev->second->pDriverTree = new PropTree( pDev->deviceDriverFeatureList(), "Driver", m_pPGDriver, static_cast<EDisplayFlags>( GetDisplayFlags() ) );
                    }
                    m_actDrvListChangedCounter = itNewDev->second->pDriverTree->Draw( false );

                    if( !itNewDev->second->pDeviceTree )
                    {
                        itNewDev->second->pDeviceTree = new PropTree( pDev->hDev(), "Device", m_pPGDevice, static_cast<EDisplayFlags>( GetDisplayFlags() ) );
                    }
                    m_actDevListChangedCounter = itNewDev->second->pDeviceTree->Draw( false );
                    if( pDev->isOpen() )
                    {
                        CreateDriverTrees( itNewDev );
                    }
                }

                PropTreeVector::size_type vSize = itNewDev->second->vDriverTrees.size();
                for( PropTreeVector::size_type i = 0; i < vSize; i++ )
                {
                    if( itNewDev->second->vDriverTrees[i] )
                    {
                        m_vTreeProps[i].ChangedCounter = itNewDev->second->vDriverTrees[i]->Draw( false );
                    }
                }

                CaptureThread* pThread = itNewDev->second->pCaptureThread;
                if( pThread )
                {
                    pThread->SetActive();
                    if( itNewDev->second->boWasLive )
                    {
                        itNewDev->second->boWasLive = pThread->GetLiveMode();
                        pThread->SetLiveMode( true );
                    }
                }
            }
        }
    }
}

//-----------------------------------------------------------------------------
void DevicePropertyHandler::SetViewMode( TViewMode mode, bool boUseHexIndices, bool boUseDisplayNames, bool boUseSelectorGrouping )
//-----------------------------------------------------------------------------
{
    if( ( mode != m_viewMode ) || ( boUseHexIndices != m_boUseHexIndices ) ||
        ( boUseDisplayNames != m_boUseDisplayNames ) || ( boUseSelectorGrouping != m_boUseSelectorGrouping ) )
    {
        m_viewMode = mode;
        m_boUseHexIndices = boUseHexIndices;
        m_boUseDisplayNames = boUseDisplayNames;
        m_boUseSelectorGrouping = boUseSelectorGrouping;
        if( m_pFullTree )
        {
            ReCreateFullTree();
        }
        else
        {
            ReCreateDriverTrees();
        }
    }
}

//-----------------------------------------------------------------------------
void DevicePropertyHandler::ReCreateDriverTrees( void )
//-----------------------------------------------------------------------------
{
    DevToTreeMap::iterator itCurDev = m_devToTreeMap.find( m_pActiveDevice );
    if( itCurDev != m_devToTreeMap.end() )
    {
        DeleteDriverTrees( itCurDev );
        CreateDriverTrees( itCurDev );
    }
}

//-----------------------------------------------------------------------------
void DevicePropertyHandler::ReCreateFullTree( void )
//-----------------------------------------------------------------------------
{
    delete m_pFullTree;
    // there has been one before, so there should be one now! WORK AROUND!!!
    m_pFullTree = new PropTree( ROOT_LIST, "Full Tree", m_pPGDevice, static_cast<EDisplayFlags>( GetDisplayFlags() ) );
    m_pFullTree->Draw( false );
}

//-----------------------------------------------------------------------------
bool DevicePropertyHandler::UpdateTree( PropTree* pTree, unsigned int& actChangedCounter, const bool boForceRedraw )
//-----------------------------------------------------------------------------
{
    if( boForceRedraw || ( actChangedCounter != pTree->GetChangedCounter() ) )
    {
        actChangedCounter = pTree->Draw( boForceRedraw );
        return true;
    }
    return false;
}

//-----------------------------------------------------------------------------
void DevicePropertyHandler::UpdateTreeMap( void )
//-----------------------------------------------------------------------------
{
    if( m_devMgr.deviceCount() != m_devToTreeMap.size() )
    {
        for( unsigned int i = 0; i < m_devMgr.deviceCount(); i++ )
        {
            DevToTreeMap::iterator it = m_devToTreeMap.find( m_devMgr[i] );
            if( it == m_devToTreeMap.end() )
            {
                PropTree* pDriverTree( m_pFullTree ? 0 : new PropTree( m_devMgr[i]->deviceDriverFeatureList(), "Driver", m_pPGDriver, static_cast<EDisplayFlags>( GetDisplayFlags() ) ) );
                PropTree* pDeviceTree( m_pFullTree ? 0 : new PropTree( m_devMgr[i]->hDev(), "Device", m_pPGDevice, static_cast<EDisplayFlags>( GetDisplayFlags() ) ) );
                m_devToTreeMap.insert( make_pair( m_devMgr[i], new DeviceData( pDriverTree, pDeviceTree ) ) );
            }
        }
    }
}

//-----------------------------------------------------------------------------
void DevicePropertyHandler::ValidateTrees( void )
//-----------------------------------------------------------------------------
{
    bool boForceRedraw = false;

    if( m_componentVisibility != GlobalDataStorage::Instance()->GetComponentVisibility() )
    {
        boForceRedraw = true;
        m_componentVisibility = GlobalDataStorage::Instance()->GetComponentVisibility();
    }

    if( m_pFullTree )
    {
        if( UpdateTree( m_pFullTree, m_fullTreeChangedCounter, boForceRedraw ) )
        {
            m_pPGDevice->Refresh();
        }
    }
    else if( m_pActiveDevData )
    {
        if( UpdateTree( m_pActiveDevData->pDriverTree, m_actDrvListChangedCounter, boForceRedraw ) )
        {
            m_pPGDriver->Refresh();
        }
        bool boUpdateDeviceTree = UpdateTree( m_pActiveDevData->pDeviceTree, m_actDevListChangedCounter, boForceRedraw );
        const PropTreeVector::size_type vSize = m_pActiveDevData->vDriverTrees.size();
        for( PropTreeVector::size_type i = 0; i < vSize; i++ )
        {
            if( m_pActiveDevData->vDriverTrees[i] )
            {
                boUpdateDeviceTree |= UpdateTree( m_pActiveDevData->vDriverTrees[i], m_vTreeProps[i].ChangedCounter, boForceRedraw );
            }
        }
        if( boUpdateDeviceTree )
        {
            m_pPGDevice->Refresh();
        }
    }
}
