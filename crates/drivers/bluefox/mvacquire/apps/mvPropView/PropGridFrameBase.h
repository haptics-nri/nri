//-----------------------------------------------------------------------------
#ifndef PropGridFrameBaseH
#define PropGridFrameBaseH PropGridFrameBaseH
//-----------------------------------------------------------------------------
#include <map>
#include <mvIMPACT_CPP/mvIMPACT_acquire.h>
#include "PropTree.h"
#include <wx/wx.h>
#include <wxPropGrid/Include/propgrid.h>

class PropData;
class wxPropertyGrid;
class wxPropertyGridEvent;

//-----------------------------------------------------------------------------
class PropGridFrameBase : public wxFrame
//-----------------------------------------------------------------------------
{
    DECLARE_EVENT_TABLE()
private:
    typedef std::map<int, wxPropertyGrid*> PropGridMap;
    PropGridMap                         m_propGrids;
    wxPropertyGrid*                     m_pPGSelected;
    wxTimer                             m_ListUpdateTimer;
protected:
    //-----------------------------------------------------------------------------
    enum TTimerEvent
    //-----------------------------------------------------------------------------
    {
        teListUpdate
    };
    //-----------------------------------------------------------------------------
    // IDs for the controls and the menu commands
    enum TMenuItemBase
    //-----------------------------------------------------------------------------
    {
        widPGDevice = 1,
        widPGDriver = 2,
        miPopUpMethExec,
        miPopUpPropForceRefresh,
        miPopUpPropRestoreDefault,
        miPopUpPropReadFromFile,
        miPopUpPropWriteToFile,
        miPopUpPropAttachCallback,
        miPopUpPropDetachCallback,
        miPopUpPropAppendValue,
        miPopUpPropDeleteValue,
        miPopUpPropSetMultiple,
        miPopUpPropSetMultiple_FixedValue,
        miPopUpPropSetMultiple_FromToRange,
        miPopUpDetailedFeatureInfo,
        mibLAST
    };
    //-----------------------------------------------------------------------------
    enum
    //-----------------------------------------------------------------------------
    {
        DEFAULT_PROP_GRID_UPDATE_PERIOD = 750
    };
    virtual void            AppendCustomPropGridExecutionErrorMessage( wxString& /*msg*/ ) const {}
    void                    ConfigureToolTipsForPropertyGrids( const bool boEnable );
    wxPropertyGrid*         CreatePropertyGrid( wxWindow* pParent, const wxSize& size = wxDefaultSize, int id = widPGDevice );
    void                    ExpandPropertyRecursively( wxPGId id );
    virtual bool            FeatureChangedCallbacksSupported( void ) const
    {
        return false;
    }
    virtual bool            FeatureHasChangedCallback( mvIMPACT::acquire::Component ) const
    {
        return false;
    }
    wxPropertyGrid*         GetPropertyGrid( void )
    {
        return m_pPGSelected;
    }
    virtual EDisplayFlags   GetDisplayFlags( void ) const
    {
        return dfNone;
    }
    void                    OnExecutePropGridMethod( wxCommandEvent& e );
    void                    OnPopUpDetailedFeatureInfo( wxCommandEvent& e );
    void                    OnPopUpPropForceRefresh( wxCommandEvent& e );
    void                    OnPopUpPropRestoreDefault( wxCommandEvent& e );
    virtual void            OnPopUpPropReadFromFile( wxCommandEvent& ) {}
    virtual void            OnPopUpPropWriteToFile( wxCommandEvent& ) {}
    virtual void            OnPopUpPropAttachCallback( wxCommandEvent& ) {}
    virtual void            OnPopUpPropDetachCallback( wxCommandEvent& ) {}
    void                    OnPopUpPropAppendValue( wxCommandEvent& e );
    void                    OnPopUpPropDeleteValue( wxCommandEvent& e );
    void                    OnPopUpPropSetMultiple_FixedValue( wxCommandEvent& e );
    void                    OnPopUpPropSetMultiple_FromToRange( wxCommandEvent& e );
    void                    OnPropertyChanged( wxPropertyGridEvent& e );
    virtual void            OnPropertyChangedCustom( wxPropertyGridEvent& ) {}
    virtual void            OnPropertyGridSelected( void ) {}
    virtual void            OnPropertyGridTimer( void ) = 0;
    void                    OnPropertyRightClicked( wxPropertyGridEvent& e );
    virtual void            OnPropertyRightClickedCustom( wxPropertyGridEvent& ) {}
    void                    OnPropertySelected( wxPropertyGridEvent& e );
    virtual void            OnPropertySelectedCustom( wxPropertyGridEvent& ) {}
    void                    OnTimer( wxTimerEvent& e );
    void                    SelectPropertyGrid( int id );
    virtual void            SelectPropertyGrid( wxPropertyGrid* pGrid );
    virtual bool            ShowPropGridMethodExecutionErrors( void ) const
    {
        return true;
    }
    void                    StartPropertyGridUpdateTimer( int period_ms );
public:
    explicit                PropGridFrameBase( wxWindowID id, const wxString& title, const wxPoint& pos, const wxSize& size );
    virtual                ~PropGridFrameBase();
    bool                    SelectPropertyInPropertyGrid( PropData* pPropData );
    void                    StartPropertyGridUpdateTimer( void )
    {
        StartPropertyGridUpdateTimer( DEFAULT_PROP_GRID_UPDATE_PERIOD );
    }
    void                    StopPropertyGridUpdateTimer( void );
    virtual void            WriteErrorMessage( const wxString& msg ) = 0;
    virtual void            WriteLogMessage( const wxString& msg, const wxTextAttr& style = wxTextAttr( *wxBLACK ) ) = 0;
};

//-----------------------------------------------------------------------------
class PropGridUpdateTimerSuspendScope
//-----------------------------------------------------------------------------
{
    PropGridFrameBase* p_;
public:
    explicit PropGridUpdateTimerSuspendScope( PropGridFrameBase* p ) : p_( p )
    {
        p_->StopPropertyGridUpdateTimer();
    }
    ~PropGridUpdateTimerSuspendScope()
    {
        p_->StartPropertyGridUpdateTimer();
    }
};


#endif // PropGridFrameBaseH
