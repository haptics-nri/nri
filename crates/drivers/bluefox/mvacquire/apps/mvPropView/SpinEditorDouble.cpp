//------------------------------------------------------------------------------
#include "PropData.h"
#include "SpinEditorDouble.h"
#include "spinctld.h"
#include <limits>
//------------------------------------------------------------------------------

using namespace std;
using namespace mvIMPACT::acquire;

//------------------------------------------------------------------------------
wxPGCustomSpinCtrlEditor* wxPGCustomSpinCtrlEditor::m_pInstance = 0;
//------------------------------------------------------------------------------

//------------------------------------------------------------------------------
wxPGCustomSpinCtrlEditor* wxPGCustomSpinCtrlEditor::Instance( void )
//------------------------------------------------------------------------------
{
    if( m_pInstance )
    {
        return m_pInstance;
    }

    m_pInstance = new wxPGCustomSpinCtrlEditor();
    wxPropertyGrid::RegisterEditorClass( m_pInstance, wxEmptyString );
    return m_pInstance;
}

//------------------------------------------------------------------------------
/// Create controls and initialize event handling.
#ifndef __WXPYTHON__
wxWindow* wxPGCustomSpinCtrlEditor::CreateControls ( wxPropertyGrid* pPropGrid, wxPGProperty* pProperty, const wxPoint& pos, const wxSize& sz, wxWindow** ) const
#else
#   if wxPROPGRID_MINOR > 2
wxPGWindowList wxPGCustomSpinCtrlEditor::CreateControls ( wxPropertyGrid* pPropGrid, wxPGProperty* pProperty, const wxPoint& pos, const wxSize& sz ) const
#   else
wxPGWindowPair wxPGCustomSpinCtrlEditor::CreateControls ( wxPropertyGrid* pPropGrid, wxPGProperty* pProperty, const wxPoint& pos, const wxSize& sz ) const
#   endif
#endif
//------------------------------------------------------------------------------
{
    // Get initial value (may be none if value is 'unspecified')
    const wxString text = pProperty->IsValueUnspecified() ? wxGetEmptyString() : pProperty->GetValueAsString( 0 );

    // Determine minimum and maximum
    double min = -1. * numeric_limits<double>::max(); // DBL_MIN is NOT what you'd expect it to be!
    double max = numeric_limits<double>::max();
    double step = 1.0;

#if wxPROPGRID_MINOR > 2
    PropertyObject* const pPropData = reinterpret_cast<PropertyObject*>( pProperty->GetClientData() );
#else
    PropertyObject* const pPropData = static_cast<PropertyObject*>( pProperty->GetClientData() );
#endif
    wxString format = wxEmptyString;
    TMode mode = mDouble;
    if( pPropData )
    {
        const Component comp = pPropData->GetComponent();
        switch( comp.type() )
        {
        case ctPropInt:
            {
                PropertyI prop( comp );
                format = ConvertedString( prop.stringFormatString() );
                min = prop.hasMinValue() ? prop.getMinValue() : numeric_limits<int>::min();
                max = prop.hasMaxValue() ? prop.getMaxValue() : numeric_limits<int>::max();
                step = prop.hasStepWidth() ? prop.getStepWidth() : 1.0;
                mode = mInt;
            }
            break;
        case ctPropInt64:
            {
                PropertyI64 prop( comp );
                format = ConvertedString( prop.stringFormatString() );
                min = prop.hasMinValue() ? prop.getMinValue() : numeric_limits<int64_type>::min();
                max = prop.hasMaxValue() ? prop.getMaxValue() : numeric_limits<int64_type>::max();
                step = prop.hasStepWidth() ? prop.getStepWidth() : 1.0;
                mode = mInt64;
            }
            break;
        case ctPropFloat:
            {
                PropertyF prop( comp );
                format = ConvertedString( prop.stringFormatString() );
                min = prop.hasMinValue() ? prop.getMinValue() : -1. * numeric_limits<double>::max(); // DBL_MIN is NOT what you'd expect it to be!
                max = prop.hasMaxValue() ? prop.getMaxValue() : numeric_limits<double>::max();
                if( prop.hasStepWidth() )
                {
                    step = prop.getStepWidth();
                }
                else
                {
                    const double range = max - min;
                    if( range <= 100. )
                    {
                        step = range / 100.;
                    }
                    else if( range <= 100000. )
                    {
                        step = range / 1000.;
                    }
                    else
                    {
                        step = 1.;
                    }
                }
            }
            break;
        default:
            {
                wxASSERT( !"unrecognized type for this editor class" );
                exit( 42 );
            }
        }
    }

    wxString valStr = pPropGrid->GetPropertyValueAsString( pProperty );
    double value = ToDouble( mode, valStr, format );

    // Use two stage creation to allow cleaner display on wxMSW
    wxSpinCtrlDbl* pEditor = new wxSpinCtrlDbl();
    pEditor->SetMode( mode );
#ifdef __WXMSW__
    pEditor->Hide();
#endif

    wxSize size( sz );
    /// \todo find out why 'sz' is too small...
    size.SetHeight( sz.GetHeight() + 6 );
    if( !pEditor->Create( pPropGrid, wxPG_SUBID1, text, pos, size, wxSP_ARROW_KEYS, min, max, value, step, 0xff, format.IsEmpty() ? wxString() : format, wxT( "wxSpinCtrlDbl" ), m_boCreateSlider ) )
    {
        wxASSERT( !"Failed wxControl::Create" );
    }
    // Connect all required events to grid's OnCustomEditorEvent
    // (all relevenat wxTextCtrl, wxComboBox and wxButton events are
    // already connected)
    pPropGrid->Connect( wxPG_SUBID1, wxEVT_COMMAND_SPINCTRL_UPDATED, ( wxObjectEventFunction )( wxEventFunction )( wxCommandEventFunction )&wxPropertyGrid::OnCustomEditorEvent );

    // This centers the control in a platform dependent manner
    pPropGrid->FixPosForTextCtrl( pEditor );

#ifdef __WXMSW__
    pEditor->Show();
#endif

    return pEditor;
}

//------------------------------------------------------------------------------
/// Copies value from property to control
void wxPGCustomSpinCtrlEditor::UpdateControl( wxPGProperty* pProperty, wxWindow* pWnd ) const
//------------------------------------------------------------------------------
{
    wxSpinCtrlDbl* pEditor = ( wxSpinCtrlDbl* )pWnd;
    wxASSERT( pEditor && pEditor->IsKindOf( CLASSINFO( wxSpinCtrlDbl ) ) );

    wxString valS( pProperty->GetValueAsString() );
    double value = ToDouble( pEditor->GetMode(), valS, pEditor->GetFormat() );
    pEditor->SetValue( value );
}

//------------------------------------------------------------------------------
/// Control's events are redirected here
bool wxPGCustomSpinCtrlEditor::OnEvent( wxPropertyGrid* pPropGrid, wxPGProperty* pProperty, wxWindow* pWnd, wxEvent& e ) const
//------------------------------------------------------------------------------
{
    if( e.GetEventType() == wxEVT_COMMAND_SPINCTRL_UPDATED )
    {
        if( CopyValueFromControl( pProperty, pWnd ) )
        {
            return true;
        }
        pPropGrid->EditorsValueWasNotModified();
    }
    else
    {
        return wxPGTextCtrlEditor::OnEvent( pPropGrid, pProperty, pWnd, e );
    }
    return false;
}

//------------------------------------------------------------------------------
/// Copies value from control to property
bool wxPGCustomSpinCtrlEditor::CopyValueFromControl( wxPGProperty* pProperty, wxWindow* pWnd ) const
//------------------------------------------------------------------------------
{
    wxSpinCtrlDbl* pEditor = ( wxSpinCtrlDbl* )pWnd;
    wxASSERT( pEditor && pEditor->IsKindOf( CLASSINFO( wxSpinCtrlDbl ) ) );
    double value = pEditor->GetValue();
    TMode mode = pEditor->GetMode();
    if( mode == mInt )
    {
        pProperty->SetValueFromString( wxString::Format( pEditor->GetFormat(), static_cast<int>( value ) ), wxPG_FULL_VALUE );
    }
    else if( mode == mInt64 )
    {
        pProperty->SetValueFromString( wxString::Format( pEditor->GetFormat(), static_cast<wxLongLong_t>( value ) ), wxPG_FULL_VALUE );
    }
    else
    {
        pProperty->SetValueFromString( wxString::Format( pEditor->GetFormat(), value ), wxPG_FULL_VALUE );
    }
    return true;
}

//------------------------------------------------------------------------------
/// Makes control look like it has unspecified value
void wxPGCustomSpinCtrlEditor::SetValueToUnspecified ( wxWindow* pWnd ) const
//------------------------------------------------------------------------------
{
    wxSpinCtrlDbl* pEditor = ( wxSpinCtrlDbl* )pWnd;
    wxASSERT( pEditor && pEditor->IsKindOf( CLASSINFO( wxSpinCtrlDbl ) ) );
    pEditor->SetValue( wxEmptyString, false );
}

//------------------------------------------------------------------------------
/// Used when control's value is wanted to set from string source
/// (obviously, not all controls can implement this properly,
///  but wxSpinCtrl can)
void wxPGCustomSpinCtrlEditor::SetControlStringValue ( wxWindow* pWnd, const wxString& txt ) const
//------------------------------------------------------------------------------
{
    wxSpinCtrlDbl* pEditor = ( wxSpinCtrlDbl* )pWnd;
    wxASSERT( pEditor && pEditor->IsKindOf( CLASSINFO( wxSpinCtrlDbl ) ) );
    pEditor->SetValue( txt, false );
}
