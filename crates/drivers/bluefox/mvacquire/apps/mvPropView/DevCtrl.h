//-----------------------------------------------------------------------------
#ifndef DevCtrlH
#define DevCtrlH DevCtrlH
//-----------------------------------------------------------------------------
#include "DevData.h"
#include "PropTree.h"
#include <wx/string.h>
#include <wx/thread.h>

class CaptureThread;
class wxPropertyGrid;
class wxWindow;

typedef std::map<mvIMPACT::acquire::Device*, DeviceData*> DevToTreeMap;

//-----------------------------------------------------------------------------
class DevicePropertyHandler
//-----------------------------------------------------------------------------
{
public:
    DevicePropertyHandler( wxPropertyGrid* pPGDriver, wxPropertyGrid* pPGDevice, bool boDisplayDebugInfo, bool boDisplayFullTree, bool boDisplayInvisibleComponents );
    ~DevicePropertyHandler();
    enum TViewMode
    {
        vmStandard,
        vmDeveloper,
        vmLAST
    };
    void                                        CloseDriver( const wxString& name );
    bool                                        DoesActiveDeviceSupportWizard( TWizardIDs wID ) const;
    mvIMPACT::acquire::Device*                  GetActiveDevice( mvIMPACT::acquire::FunctionInterface** ppFI = 0, const mvIMPACT::acquire::Statistics** ppS = 0, CaptureThread** ppCT = 0 ) const;
    mvIMPACT::acquire::Property                 GetActiveDeviceAcquisitionMode( void ) const;
    mvIMPACT::acquire::PropertyI64              GetActiveDeviceAcquisitionFrameCount( void ) const;
    const WizardFeatureMap                      GetActiveDeviceSupportedWizards( void ) const;
    bool                                        GetDeviceData( mvIMPACT::acquire::Device* pDev, mvIMPACT::acquire::FunctionInterface** ppFI = 0, const mvIMPACT::acquire::Statistics** ppS = 0, CaptureThread** ppCT = 0 ) const;
    const mvIMPACT::acquire::DeviceManager&     GetDevMgr( void ) const
    {
        return m_devMgr;
    }
    EDisplayFlags                               GetDisplayFlags( void ) const;
    TViewMode                                   GetViewMode( void ) const
    {
        return m_viewMode;
    }
    void                                        OpenDriver( const wxString& name, wxWindow* pParentWindow, unsigned int pendingImageQueueDepth );
    void                                        SetActiveDevice( const wxString& name );
    void                                        SetViewMode( TViewMode mode, bool boUseHexIndices, bool boUseDisplayNames, bool boUseSelectorGrouping );
    void                                        ValidateTrees( void );
private:
    unsigned int                                m_actDevListChangedCounter;
    unsigned int                                m_actDrvListChangedCounter;
    bool                                        m_boDisplayDebugInfo;
    bool                                        m_boDisplayInvisibleComponents;
    mvIMPACT::acquire::TComponentVisibility     m_componentVisibility;
    unsigned int                                m_fullTreeChangedCounter;
    PropTree*                                   m_pFullTree;
    mvIMPACT::acquire::DeviceManager            m_devMgr;
    DevToTreeMap                                m_devToTreeMap;
    const DeviceData*                           m_pActiveDevData;
    mvIMPACT::acquire::Device*                  m_pActiveDevice;
    wxPropertyGrid*                             m_pPGDevice;
    wxPropertyGrid*                             m_pPGDriver;
    mutable wxCriticalSection                   m_critSect;
    TViewMode                                   m_viewMode;
    bool                                        m_boUseHexIndices;
    bool                                        m_boUseDisplayNames;
    bool                                        m_boUseSelectorGrouping;
    static void                                 AddFeatureToSetIfValid( std::set<HOBJ>& s, Component& c );
    void                                        GetInterfaceClasses( const DeviceData* p, mvIMPACT::acquire::FunctionInterface** ppFI = 0, const mvIMPACT::acquire::Statistics** ppS = 0, CaptureThread** ppCT = 0 ) const;
    void                                        ReCreateDriverTrees( void );
    void                                        ReCreateFullTree( void );
    void                                        CheckForWizards( mvIMPACT::acquire::Device* pDev, DeviceData* pDevData );
    bool                                        UpdateTree( PropTree* pTree, unsigned int& actChangedCounter, const bool boForceRedraw );
    void                                        UpdateTreeMap( void );
    void                                        CreateDriverTrees( DevToTreeMap::iterator& it );
    void                                        DeleteDriverTrees( DevToTreeMap::iterator& it );
    void                                        PrepareTreeArray( void );
    void                                        PrepareTreeArrayCommon( void );

    struct TreeProp
    {
        TreeProp( const std::string& listName, const std::string& displayedName ) : ListName( listName ),
            DisplayName( displayedName ), ChangedCounter( 0 ), ListType( mvIMPACT::acquire::dltUndefined ) {}
        TreeProp( mvIMPACT::acquire::TDeviceListType listType, const std::string& displayedName ) : ListName( "" ),
            DisplayName( displayedName ), ChangedCounter( 0 ), ListType( listType ) {}
        std::string                         ListName;
        std::string                         DisplayName;
        unsigned int                        ChangedCounter;
        mvIMPACT::acquire::TDeviceListType  ListType;
    };
    std::vector<TreeProp>   m_vTreeProps;
};

#endif // DevCtrlH
