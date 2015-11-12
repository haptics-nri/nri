//-----------------------------------------------------------------------------
#ifndef SpinEditorDoubleH
#define SpinEditorDoubleH SpinEditorDoubleH
//-----------------------------------------------------------------------------

#include <wxPropGrid/Include/propgrid.h>
#include <wxPropGrid/Include/propdev.h>
#if wxPROPGRID_MINOR > 2
#   ifdef SWIG
% import < wx / propgrid / editors.h >
#   else
#       include <wx/propgrid/editors.h>
#   endif
#endif

//------------------------------------------------------------------------------
class wxPGCustomSpinCtrlEditor : public wxPGTextCtrlEditor
//------------------------------------------------------------------------------
{
    explicit wxPGCustomSpinCtrlEditor() : wxPGTextCtrlEditor(), m_boCreateSlider( true ) {}
    static wxPGCustomSpinCtrlEditor* m_pInstance;
    bool m_boCreateSlider;
public:
    // See below for short explanations of what these are suppposed to do.
#ifndef __WXPYTHON__
    virtual wxWindow*                   CreateControls( wxPropertyGrid* pPropGrid, wxPGProperty* pProperty, const wxPoint& pos, const wxSize& sz, wxWindow** ppSecondary ) const;
#else
#   if wxPROPGRID_MINOR > 2
    virtual wxPGWindowList          CreateControls( wxPropertyGrid* pPropGrid, wxPGProperty* pProperty, const wxPoint& pos, const wxSize& sz ) const;
#   else
    virtual wxPGWindowPair          CreateControls( wxPropertyGrid* pPropGrid, wxPGProperty* pProperty, const wxPoint& pos, const wxSize& sz ) const;
#   endif
#endif
    void                        ConfigureControlsCreation( bool boCreateSlider )
    {
        m_boCreateSlider = boCreateSlider;
    }
    virtual bool                        CopyValueFromControl( wxPGProperty* pProperty, wxWindow* pWnd ) const;
    virtual wxPG_CONST_WXCHAR_PTR       GetName( void ) const
    {
        return wxT( "PGCustomSpinCtrlEditor" );
    }
#ifndef SWIG
    static wxPGCustomSpinCtrlEditor*    Instance( void );
#endif
    virtual void                        SetValueToUnspecified( wxWindow* pWnd ) const;
    virtual void                        SetControlStringValue( wxWindow* pWnd, const wxString& txt ) const;
    virtual bool                        OnEvent( wxPropertyGrid* pPropGrid, wxPGProperty* pProperty, wxWindow* pWnd, wxEvent& e ) const;
    virtual void                        OnFocus( wxPGProperty*, wxWindow* ) const {} // must overload the function from the base class!
    virtual void                        UpdateControl( wxPGProperty* pProperty, wxWindow* pWnd ) const;
};

#endif // SpinEditorDoubleH
