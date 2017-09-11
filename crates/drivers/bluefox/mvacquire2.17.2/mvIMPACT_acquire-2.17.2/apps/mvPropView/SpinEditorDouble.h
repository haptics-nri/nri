//-----------------------------------------------------------------------------
#ifndef SpinEditorDoubleH
#define SpinEditorDoubleH SpinEditorDoubleH
//-----------------------------------------------------------------------------
#include <wx/wx.h>
#if wxMAJOR_VERSION < 3
#   error "You need at least Version 3.0.0 of wxWidgets to compile this application"
#endif // #if wxMAJOR_VERSION < 3
#include <wx/propgrid/propgrid.h>

//------------------------------------------------------------------------------
class wxPGCustomSpinCtrlEditor : public wxPGTextCtrlEditor
//------------------------------------------------------------------------------
{
    static wxPGCustomSpinCtrlEditor* m_pInstance;
    static wxPGEditor* m_pEditor;
    bool m_boCreateSlider;

    explicit wxPGCustomSpinCtrlEditor() : wxPGTextCtrlEditor(), m_boCreateSlider( true ) {}
    bool                                CopyValueFromControl( wxPGProperty* pProperty, wxWindow* pWnd ) const;
    wxString                            GetValueFromControlAsString( wxWindow* pWnd ) const;
public:
    virtual wxPGWindowList              CreateControls( wxPropertyGrid* pPropGrid, wxPGProperty* pProperty, const wxPoint& pos, const wxSize& sz ) const;
    void                                ConfigureControlsCreation( bool boCreateSlider )
    {
        m_boCreateSlider = boCreateSlider;
    }
    wxPGEditor*                         GetEditor( void ) const
    {
        return m_pEditor;
    }
    virtual wxString                    GetName( void ) const
    {
        return wxT( "CustomSpinCtrl" );
    }
    virtual bool                        GetValueFromControl( wxVariant& variant, wxPGProperty* pProperty, wxWindow* pWnd ) const;
#ifndef SWIG
    static wxPGCustomSpinCtrlEditor*    Instance( void );
#endif
    virtual bool                        OnEvent( wxPropertyGrid* pPropGrid, wxPGProperty* pProperty, wxWindow* pWnd, wxEvent& e ) const;
    virtual void                        OnFocus( wxPGProperty* pProperty, wxWindow* pWnd ) const;
    virtual void                        SetValueToUnspecified( wxPGProperty* pProperty, wxWindow* pWnd ) const;
    virtual void                        SetControlStringValue( wxPGProperty* pProperty, wxWindow* pWnd, const wxString& txt ) const;
    virtual void                        UpdateControl( wxPGProperty* pProperty, wxWindow* pWnd ) const;
};

#endif // SpinEditorDoubleH
