#include <limits>
#include <algorithm>
#include <functional>
#include "PropData.h"
#include "PropGridFrameBase.h"
#include "ValuesFromUserDlg.h"
#include <wx/spinctrl.h>

using namespace mvIMPACT::acquire;
using namespace std;

//-----------------------------------------------------------------------------
template<class _Ty>
void DeleteElement( _Ty& data )
//-----------------------------------------------------------------------------
{
    delete data;
    data = 0;
}

//-----------------------------------------------------------------------------
/// \brief Used for internal refresh calls...
template<typename _Ty>
void DummyRead( Component comp )
//-----------------------------------------------------------------------------
{
    _Ty prop( comp );
    vector<typename _Ty::value_type> v;
    prop.read( v, 0 );
}

//=============================================================================
//================= Implementation PropGridFrameBase ==========================
//=============================================================================
BEGIN_EVENT_TABLE( PropGridFrameBase, wxFrame )
    EVT_TIMER( wxID_ANY, PropGridFrameBase::OnTimer )
    EVT_MENU( miPopUpPropForceRefresh, PropGridFrameBase::OnPopUpPropForceRefresh )
    EVT_MENU( miPopUpPropRestoreDefault, PropGridFrameBase::OnPopUpPropRestoreDefault )
    EVT_MENU( miPopUpPropReadFromFile, PropGridFrameBase::OnPopUpPropReadFromFile )
    EVT_MENU( miPopUpPropWriteToFile, PropGridFrameBase::OnPopUpPropWriteToFile )
    EVT_MENU( miPopUpPropAttachCallback, PropGridFrameBase::OnPopUpPropAttachCallback )
    EVT_MENU( miPopUpPropDetachCallback, PropGridFrameBase::OnPopUpPropDetachCallback )
    EVT_MENU( miPopUpPropAppendValue, PropGridFrameBase::OnPopUpPropAppendValue )
    EVT_MENU( miPopUpPropDeleteValue, PropGridFrameBase::OnPopUpPropDeleteValue )
    EVT_MENU( miPopUpPropSetMultiple_FixedValue, PropGridFrameBase::OnPopUpPropSetMultiple_FixedValue )
    EVT_MENU( miPopUpPropSetMultiple_FromToRange, PropGridFrameBase::OnPopUpPropSetMultiple_FromToRange )
    EVT_MENU( miPopUpMethExec, PropGridFrameBase::OnExecutePropGridMethod )
    EVT_MENU( miPopUpDetailedFeatureInfo, PropGridFrameBase::OnPopUpDetailedFeatureInfo )
    EVT_PG_CHANGED( widPGDevice, PropGridFrameBase::OnPropertyChanged )
    EVT_PG_CHANGED( widPGDriver, PropGridFrameBase::OnPropertyChanged )
    EVT_PG_RIGHT_CLICK( widPGDevice, PropGridFrameBase::OnPropertyRightClicked )
    EVT_PG_RIGHT_CLICK( widPGDriver, PropGridFrameBase::OnPropertyRightClicked )
    EVT_PG_SELECTED( widPGDevice, PropGridFrameBase::OnPropertySelected )
    EVT_PG_SELECTED( widPGDriver, PropGridFrameBase::OnPropertySelected )
    EVT_BUTTON( widPGDevice, PropGridFrameBase::OnExecutePropGridMethod )
    EVT_BUTTON( widPGDriver, PropGridFrameBase::OnExecutePropGridMethod )
END_EVENT_TABLE()

//-----------------------------------------------------------------------------
PropGridFrameBase::PropGridFrameBase( wxWindowID id, const wxString& title, const wxPoint& pos, const wxSize& size )
    : wxFrame( 0, id, title, pos, size ), m_propGrids(), m_pPGSelected( 0 )
//-----------------------------------------------------------------------------
{

}

//-----------------------------------------------------------------------------
PropGridFrameBase::~PropGridFrameBase()
//-----------------------------------------------------------------------------
{
    StopPropertyGridUpdateTimer();
}

//-----------------------------------------------------------------------------
void PropGridFrameBase::ConfigureToolTipsForPropertyGrids( const bool boEnable )
//-----------------------------------------------------------------------------
{
    PropGridMap::iterator it = m_propGrids.begin();
    const PropGridMap::iterator itEND = m_propGrids.end();
    while( it != itEND )
    {
        long style = it->second->GetExtraStyle();
        if( boEnable )
        {
            it->second->SetExtraStyle( style | wxPG_EX_HELP_AS_TOOLTIPS );
        }
        else
        {
            it->second->SetExtraStyle( style & ~wxPG_EX_HELP_AS_TOOLTIPS );
        }
        ++it;
    }
}

//-----------------------------------------------------------------------------
wxPropertyGrid* PropGridFrameBase::CreatePropertyGrid( wxWindow* pParent, const wxSize& size /*= wxDefaultSize*/, int id /* = widPGDevice */ )
//-----------------------------------------------------------------------------
{
    PropGridMap::iterator it = m_propGrids.find( id );
    if( it != m_propGrids.end() )
    {
        return it->second;
    }

    wxPropertyGrid* p = new wxPropertyGrid( pParent, id, wxDefaultPosition, size, wxPG_SPLITTER_AUTO_CENTER | wxPG_TOOLTIPS | wxTAB_TRAVERSAL | wxSUNKEN_BORDER );
#if wxPROPGRID_MINOR < 3
    p->Compact( true );
#endif
    p->SetLineColour( wxColour( 128, 128, 128 ) );
    m_propGrids.insert( make_pair( id, p ) );
    return p;
}

//-----------------------------------------------------------------------------
void PropGridFrameBase::ExpandPropertyRecursively( wxPGId id )
//-----------------------------------------------------------------------------
{
    if( wxPGIdIsOk( id ) && m_pPGSelected )
    {
        if( wxPGIdToPtr( id )->GetParent() )
        {
            ExpandPropertyRecursively( wxPGIdToPtr( id )->GetParent()->GetId() );
        }
        GetPropertyGrid()->Expand( id );
    }
}

//-----------------------------------------------------------------------------
void PropGridFrameBase::OnExecutePropGridMethod( wxCommandEvent& )
//-----------------------------------------------------------------------------
{
    wxPGId id = GetPropertyGrid()->GetSelectedProperty();
    if( wxPGIdIsOk( id ) )
    {
        PropData* pPropData = static_cast<PropData*>( wxPGIdToPtr( id )->GetClientData() );
        // make sure we are actually dealing with a method object
        MethodObject* pMethod = dynamic_cast<MethodObject*>( pPropData );
        if( pMethod )
        {
            int callResult = DMR_NO_ERROR;
            WriteLogMessage( pMethod->Call( callResult ) );
            WriteLogMessage( wxT( '\n' ) );
            if( ( callResult != DMR_NO_ERROR ) && ShowPropGridMethodExecutionErrors() )
            {
                wxString errorString( wxString::Format( wxT( "An error occurred while executing function %s. Error code: %d (%s))." ), pMethod->FriendlyName().c_str(), callResult, ImpactAcquireException::getErrorCodeAsString( callResult ).c_str() ) );
                AppendCustomPropGridExecutionErrorMessage( errorString );
                wxMessageDialog errorDlg( NULL, errorString, wxT( "Method Execution Failed" ), wxOK | wxICON_INFORMATION );
                errorDlg.ShowModal();
            }
        }
        // this branch is also reached by custom editors, that are not methods(e.g. dirSelector Dialog, thus in order to avoid confusion
        //
        //else
        //{
        //  WriteErrorMessage( wxString::Format( wxT("There was an 'execute' message for an object(%s) that is not a method.\n"), wxPGIdToPtr(id)->GetLabel().c_str() ) );
        //}
    }
}

//-----------------------------------------------------------------------------
void PropGridFrameBase::OnPopUpDetailedFeatureInfo( wxCommandEvent& )
//-----------------------------------------------------------------------------
{
    wxPGId id = GetPropertyGrid()->GetSelectedProperty();
    if( wxPGIdIsOk( id ) )
    {
        PropData* pData = static_cast<PropData*>( wxPGIdToPtr( id )->GetClientData() );
        if( pData )
        {
            DetailedFeatureInfoDlg dlg( this, pData->GetComponent() );
            dlg.ShowModal();
        }
    }
}

//-----------------------------------------------------------------------------
void PropGridFrameBase::OnPopUpPropAppendValue( wxCommandEvent& )
//-----------------------------------------------------------------------------
{
    wxPGId id = GetPropertyGrid()->GetSelectedProperty();
    if( wxPGIdIsOk( id ) )
    {
        PropData* pData = static_cast<PropData*>( wxPGIdToPtr( id )->GetClientData() );
        if( pData )
        {
            VectorPropertyObject* pProp = dynamic_cast<VectorPropertyObject*>( pData );
            if( pProp )
            {
                Property prop( pProp->GetComponent().hObj() );
                prop.resizeValArray( prop.valCount() + 1 );
            }
        }
    }
}

//-----------------------------------------------------------------------------
void PropGridFrameBase::OnPopUpPropDeleteValue( wxCommandEvent& )
//-----------------------------------------------------------------------------
{
    wxPGId id = GetPropertyGrid()->GetSelectedProperty();
    if( wxPGIdIsOk( id ) )
    {
        PropData* pData = static_cast<PropData*>( wxPGIdToPtr( id )->GetClientData() );
        if( pData )
        {
            VectorPropertyObject* pProp = dynamic_cast<VectorPropertyObject*>( pData );
            if( pProp )
            {
                Property prop( pProp->GetComponent().hObj() );
                pProp->RemoveValue( prop.valCount() - 1 );
            }
        }
    }
}

//-----------------------------------------------------------------------------
void PropGridFrameBase::OnPopUpPropForceRefresh( wxCommandEvent& )
//-----------------------------------------------------------------------------
{
    wxPGId id = GetPropertyGrid()->GetSelectedProperty();
    if( wxPGIdIsOk( id ) )
    {
        Component comp = static_cast<PropData*>( wxPGIdToPtr( id )->GetClientData() )->GetComponent();
        switch( comp.type() )
        {
        case ctPropFloat:
            DummyRead<PropertyF>( comp );
            break;
        case ctPropInt:
            DummyRead<PropertyI>( comp );
            break;
        case ctPropInt64:
            DummyRead<PropertyI64>( comp );
            break;
        case ctPropPtr:
            DummyRead<PropertyPtr>( comp );
            break;
        case ctPropString:
            DummyRead<PropertyS>( comp );
            break;
        default:
            WriteErrorMessage( wxString::Format( wxT( "Unhandled data type in function %s detected. Component %s is of type %s.\n" ),
                                                 ConvertedString( __FUNCTION__ ).c_str(),
                                                 static_cast<PropData*>( wxPGIdToPtr( id )->GetClientData() )->GetDisplayName( GetDisplayFlags() ).c_str(),
                                                 ConvertedString( comp.typeAsString() ).c_str() ) );
            break;
        }
        if( comp.isDefault() )
        {
            GetPropertyGrid()->ClearModifiedStatus( id );
        }
    }
}

//-----------------------------------------------------------------------------
void PropGridFrameBase::OnPopUpPropRestoreDefault( wxCommandEvent& )
//-----------------------------------------------------------------------------
{
    wxPGId id = GetPropertyGrid()->GetSelectedProperty();
    if( wxPGIdIsOk( id ) )
    {
        Component comp = static_cast<PropData*>( wxPGIdToPtr( id )->GetClientData() )->GetComponent();
        if( comp.type() == ctList )
        {
            wxMessageDialog AreYouSureDlg( NULL,
                                           wxString::Format( wxT( "All properties in the list '%s' will be set to default.\n" ),
                                                   static_cast<PropData*>( wxPGIdToPtr( id )->GetClientData() )->GetDisplayName( GetDisplayFlags() ).c_str() ),
                                           wxT( "Warning" ),
                                           wxNO_DEFAULT | wxYES_NO | wxICON_INFORMATION );

            if( AreYouSureDlg.ShowModal() != wxID_YES )
            {
                return;
            }
        }
        if( comp.isDefault() )
        {
            WriteLogMessage( wxString::Format( wxT( "Element '%s' is already set to the default value.\n" ),
                                               static_cast<PropData*>( wxPGIdToPtr( id )->GetClientData() )->GetDisplayName( GetDisplayFlags() ).c_str() ) );
        }
        else
        {
            try
            {
                comp.restoreDefault();
            }
            catch( const ImpactAcquireException& e )
            {
                wxMessageDialog errorDlg( NULL, wxString::Format( wxT( "Failed to restore the default for feature '%s'(Error: %s)" ), ConvertedString( comp.name() ).c_str(), ConvertedString( e.getErrorCodeAsString() ).c_str() ), wxT( "Error" ), wxOK | wxICON_INFORMATION );
                errorDlg.ShowModal();
            }
        }
        GetPropertyGrid()->ClearModifiedStatus( id );
    }
}

//-----------------------------------------------------------------------------
void PropGridFrameBase::OnPopUpPropSetMultiple_FixedValue( wxCommandEvent& )
//-----------------------------------------------------------------------------
{
    wxPGId id = GetPropertyGrid()->GetSelectedProperty();
    if( wxPGIdIsOk( id ) )
    {
        PropData* pData = static_cast<PropData*>( wxPGIdToPtr( id )->GetClientData() );
        if( pData )
        {
            VectorPropertyObject* pProp = dynamic_cast<VectorPropertyObject*>( pData );
            if( pProp && ( pProp->GetComponent().type() == ctPropInt ) )
            {
                vector<ValueData*> v;
                try
                {
                    PropertyI prop( pProp->GetComponent() );
                    const unsigned int valCnt = prop.valCount();
                    v.push_back( new ValueRangeData( wxString( wxT( "First value to set" ) ), 0, valCnt - 1, 1, 0 ) );
                    v.push_back( new ValueRangeData( wxString( wxT( "Last value to set" ) ), 0, valCnt - 1, 1, valCnt - 1 ) );
                    if( prop.hasDict() )
                    {
                        wxArrayString stringArray;
                        vector<pair<string, int> > translationDict;
                        prop.getTranslationDict( translationDict );
                        vector<pair<string, int> >::size_type vSize = translationDict.size();
                        for( vector<pair<string, int> >::size_type i = 0; i < vSize; i++ )
                        {
                            stringArray.push_back( ConvertedString( translationDict[i].first ) );
                        }
                        v.push_back( new ValueChoiceData( wxString( wxT( "Value" ) ), stringArray ) );
                    }
                    else
                    {
                        v.push_back( new ValueRangeData( wxString( wxT( "Value" ) ),
                                                         prop.hasMinValue() ? prop.getMinValue() : numeric_limits<int>::min(),
                                                         prop.hasMaxValue() ? prop.getMaxValue() : numeric_limits<int>::max(),
                                                         prop.hasStepWidth() ? prop.getStepWidth() : 1,
                                                         prop.hasMinValue() ? prop.getMinValue() : numeric_limits<int>::min() ) );
                    }
                    ValuesFromUserDlg dlg( this, wxString( wxT( "Select the value and range to apply" ) ), v );
                    if( dlg.ShowModal() == wxID_OK )
                    {
                        const vector<wxControl*>& resultData = dlg.GetUserInputControls();
                        wxASSERT( dynamic_cast<wxSpinCtrl*>( resultData[0] ) && dynamic_cast<wxSpinCtrl*>( resultData[1] ) );
                        int first = dynamic_cast<wxSpinCtrl*>( resultData[0] )->GetValue();
                        int last = dynamic_cast<wxSpinCtrl*>( resultData[1] )->GetValue();
                        if( first <= last )
                        {
                            try
                            {
                                if( prop.hasDict() )
                                {
                                    vector<string> sequence( last - first + 1, string( dynamic_cast<wxComboBox*>( resultData[2] )->GetValue().mb_str() ) );
                                    prop.writeS( sequence, first );
                                }
                                else
                                {
                                    vector<int> sequence( last - first + 1, dynamic_cast<wxSpinCtrl*>( resultData[2] )->GetValue() );
                                    prop.write( sequence, true, first );
                                }
                            }
                            catch( const ImpactAcquireException& e )
                            {
                                wxMessageDialog errorDlg( NULL, wxString::Format( wxT( "Couldn't set value range(Error: %s)" ), ConvertedString( e.getErrorCodeAsString() ).c_str() ), wxT( "Error" ), wxOK | wxICON_INFORMATION );
                                errorDlg.ShowModal();
                            }
                        }
                        else
                        {
                            WriteErrorMessage( wxString::Format( wxT( "%s(%d): Invalid input data!\n" ), ConvertedString( __FUNCTION__ ).c_str(), __LINE__ ) );
                        }
                    }
                }
                catch( const ImpactAcquireException& e )
                {
                    wxMessageDialog errorDlg( NULL, wxString::Format( wxT( "Internal problem(Error: %s)" ), ConvertedString( e.getErrorCodeAsString() ).c_str() ), wxT( "Error" ), wxOK | wxICON_INFORMATION );
                    errorDlg.ShowModal();
                }
                for_each( v.begin(), v.end(), ptr_fun( DeleteElement<ValueData*> ) );
            }
            else
            {
                WriteErrorMessage( wxString::Format( wxT( "%s(%d): Element '%s' doesn't seem to be a vector property!\n" ),
                                                     ConvertedString( __FUNCTION__ ).c_str(), __LINE__,
                                                     pData->GetDisplayName( GetDisplayFlags() ).c_str() ) );
            }
        }
        else
        {
            WriteErrorMessage( wxString::Format( wxT( "%s(%d): Invalid client data detected!\n" ), ConvertedString( __FUNCTION__ ).c_str(), __LINE__ ) );
        }
    }
    else
    {
        WriteErrorMessage( wxString::Format( wxT( "%s(%d): No item selected!\n" ), ConvertedString( __FUNCTION__ ).c_str(), __LINE__ ) );
    }
}

//-----------------------------------------------------------------------------
void PropGridFrameBase::OnPopUpPropSetMultiple_FromToRange( wxCommandEvent& )
//-----------------------------------------------------------------------------
{
    wxPGId id = GetPropertyGrid()->GetSelectedProperty();
    if( wxPGIdIsOk( id ) )
    {
        PropData* pData = static_cast<PropData*>( wxPGIdToPtr( id )->GetClientData() );
        if( pData )
        {
            VectorPropertyObject* pProp = dynamic_cast<VectorPropertyObject*>( pData );
            if( pProp && ( pProp->GetComponent().type() == ctPropInt ) )
            {
                vector<ValueData*> v;
                try
                {
                    PropertyI prop( pProp->GetComponent() );
                    const unsigned int valCnt = prop.valCount();
                    v.push_back( new ValueRangeData( wxString( wxT( "First value to set" ) ), 0, valCnt - 1, 1, 0 ) );
                    v.push_back( new ValueRangeData( wxString( wxT( "Last value to set" ) ), 0, valCnt - 1, 1, valCnt - 1 ) );
                    wxASSERT( !prop.hasDict() );
                    v.push_back( new ValueRangeData( wxString( wxT( "Start value" ) ),
                                                     prop.hasMinValue() ? prop.getMinValue() : numeric_limits<int>::min(),
                                                     prop.hasMaxValue() ? prop.getMaxValue() : numeric_limits<int>::max(),
                                                     prop.hasStepWidth() ? prop.getStepWidth() : 1,
                                                     prop.hasMinValue() ? prop.getMinValue() : numeric_limits<int>::min() ) );
                    v.push_back( new ValueRangeData( wxString( wxT( "End value" ) ),
                                                     prop.hasMinValue() ? prop.getMinValue() : numeric_limits<int>::min(),
                                                     prop.hasMaxValue() ? prop.getMaxValue() : numeric_limits<int>::max(),
                                                     prop.hasStepWidth() ? prop.getStepWidth() : 1,
                                                     prop.hasMinValue() ? prop.getMaxValue() : numeric_limits<int>::max() ) );
                    ValuesFromUserDlg dlg( this, wxString( wxT( "Select the value and range to apply" ) ), v );
                    if( dlg.ShowModal() == wxID_OK )
                    {
                        const vector<wxControl*>& resultData = dlg.GetUserInputControls();
                        wxASSERT( dynamic_cast<wxSpinCtrl*>( resultData[0] ) && dynamic_cast<wxSpinCtrl*>( resultData[1] ) &&
                                  dynamic_cast<wxSpinCtrl*>( resultData[2] ) && dynamic_cast<wxSpinCtrl*>( resultData[3] ) );
                        int first = dynamic_cast<wxSpinCtrl*>( resultData[0] )->GetValue();
                        int last = dynamic_cast<wxSpinCtrl*>( resultData[1] )->GetValue();
                        int startVal = dynamic_cast<wxSpinCtrl*>( resultData[2] )->GetValue();
                        int endVal = dynamic_cast<wxSpinCtrl*>( resultData[3] )->GetValue();
                        if( first <= last )
                        {
                            try
                            {
                                vector<int> sequence;
                                int valuesToProcess = last - first + 1;
                                double increment = static_cast<double>( endVal - startVal ) / static_cast<double>( valuesToProcess - 1 );
                                for( int i = 0; i < valuesToProcess; i++ )
                                {
                                    int offset = static_cast<int>( i * increment );
                                    if( i * increment - offset > 0.5 )
                                    {
                                        ++offset;
                                    }
                                    sequence.push_back( startVal + offset );
                                }
                                prop.write( sequence, true, first );
                            }
                            catch( const ImpactAcquireException& e )
                            {
                                wxMessageDialog errorDlg( NULL, wxString::Format( wxT( "Couldn't set value range(Error: %s)" ), ConvertedString( e.getErrorCodeAsString() ).c_str() ), wxT( "Error" ), wxOK | wxICON_INFORMATION );
                                errorDlg.ShowModal();
                            }
                        }
                        else
                        {
                            WriteErrorMessage( wxString::Format( wxT( "%s(%d): Invalid input data!\n" ), ConvertedString( __FUNCTION__ ).c_str(), __LINE__ ) );
                        }
                    }
                }
                catch( const ImpactAcquireException& e )
                {
                    wxMessageDialog errorDlg( NULL, wxString::Format( wxT( "Internal problem(Error: %s)" ), ConvertedString( e.getErrorCodeAsString() ).c_str() ), wxT( "Error" ), wxOK | wxICON_INFORMATION );
                    errorDlg.ShowModal();
                }
                for_each( v.begin(), v.end(), ptr_fun( DeleteElement<ValueData*> ) );
            }
            else
            {
                WriteErrorMessage( wxString::Format( wxT( "%s(%d): Element '%s' doesn't seem to be a vector property!\n" ),
                                                     ConvertedString( __FUNCTION__ ).c_str(),
                                                     __LINE__,
                                                     pData->GetDisplayName( GetDisplayFlags() ).c_str() ) );
            }
        }
        else
        {
            WriteErrorMessage( wxString::Format( wxT( "%s(%d): Invalid client data detected!\n" ), ConvertedString( __FUNCTION__ ).c_str(), __LINE__ ) );
        }
    }
    else
    {
        WriteErrorMessage( wxString::Format( wxT( "%s(%d): No item selected!\n" ), ConvertedString( __FUNCTION__ ).c_str(), __LINE__ ) );
    }
}

//-----------------------------------------------------------------------------
void PropGridFrameBase::OnPropertyChanged( wxPropertyGridEvent& e )
//-----------------------------------------------------------------------------
{
    PropData* prop_data = static_cast<PropData*>( wxPGIdToPtr( e.GetProperty() )->GetClientData() );
    if( prop_data )
    {
        prop_data->UpdatePropData();
    }
    OnPropertyChangedCustom( e );
}

//-----------------------------------------------------------------------------
void PropGridFrameBase::OnPropertyRightClicked( wxPropertyGridEvent& e )
//-----------------------------------------------------------------------------
{
    PropData* prop_data = static_cast<PropData*>( wxPGIdToPtr( e.GetProperty() )->GetClientData() );
    if( prop_data )
    {
        bool boIsProp = prop_data->GetComponent().isProp();
        if( boIsProp || prop_data->GetComponent().isList() )
        {
            wxMenu menu( wxT( "" ) );
            menu.Append( miPopUpPropForceRefresh, wxT( "&Force Refresh" ) )->Enable( boIsProp );
            bool boWriteable = prop_data->GetComponent().isWriteable();
            menu.Append( miPopUpPropRestoreDefault, wxT( "&Restore Default" ) )->Enable( boWriteable );
            menu.Append( miPopUpDetailedFeatureInfo, wxT( "Detailed Feature Information" ) )->Enable( true );
            menu.AppendSeparator();
            if( FeatureChangedCallbacksSupported() )
            {
                const bool boFeatureHasCallbackRegistered = FeatureHasChangedCallback( prop_data->GetComponent() );
                menu.Append( miPopUpPropAttachCallback, wxT( "Attach Callback" ) )->Enable( !boFeatureHasCallbackRegistered );
                menu.Append( miPopUpPropDetachCallback, wxT( "Detach Callback" ) )->Enable( boFeatureHasCallbackRegistered );
            }
            menu.AppendSeparator();
            // The 'GetComponent().type()' check is only needed because some drivers with versions < 1.12.33
            // did incorrectly specify the 'cfContainsBinaryData' flag even though the data type was not 'ctPropString'...
            bool boContainsBinaryData = ( ( prop_data->GetComponent().type() == ctPropString ) && ( prop_data->GetComponent().flags() & cfContainsBinaryData ) ) != 0;
            menu.Append( miPopUpPropReadFromFile, wxT( "Read File Into Property Value" ) )->Enable( boContainsBinaryData );
            menu.Append( miPopUpPropWriteToFile, wxT( "Write Property Value To File" ) )->Enable( boContainsBinaryData );
            bool boAppendValuePossible = false;
            bool boDeleteLastValuePossible = false;
            bool boMultiToConstPossible = false;
            bool boMultiToRangePossible = false;
            wxPGId id = e.GetPropertyGrid()->GetSelectedProperty();
            if( wxPGIdIsOk( id ) )
            {
                PropData* pData = static_cast<PropData*>( wxPGIdToPtr( id )->GetClientData() );
                if( pData )
                {
                    VectorPropertyObject* pProp = dynamic_cast<VectorPropertyObject*>( pData );
                    if( pProp )
                    {
                        Property driverProp( pProp->GetComponent() );
                        if( !( prop_data->GetComponent().flags() & cfFixedSize ) )
                        {
                            const unsigned int valCnt = driverProp.valCount();
                            const unsigned int maxValCnt = driverProp.maxValCount();
                            boAppendValuePossible = ( ( maxValCnt > valCnt ) && boWriteable ) ? true : false;
                            boDeleteLastValuePossible = ( ( valCnt > 1 ) && boWriteable ) ? true : false;
                        }
                        if( driverProp.type() == ctPropInt )
                        {
                            boMultiToConstPossible = boWriteable;
                            if( !driverProp.hasDict() )
                            {
                                boMultiToRangePossible = boWriteable;
                            }
                        }
                    }
                }
            }
            menu.Append( miPopUpPropAppendValue, wxT( "&Append Value" ) )->Enable( boAppendValuePossible );
            menu.Append( miPopUpPropDeleteValue, wxT( "&Delete Last Value" ) )->Enable( boDeleteLastValuePossible );
            wxMenu* pSubMenu = new wxMenu( wxT( "" ) );
            pSubMenu->Append( miPopUpPropSetMultiple_FixedValue, wxT( "&To A Constant Value" ) )->Enable( boMultiToConstPossible );
            pSubMenu->Append( miPopUpPropSetMultiple_FromToRange, wxT( "Via A User Defined &Value Range" ) )->Enable( boMultiToRangePossible );
            menu.Append( wxID_ANY, wxT( "Set Multiple Elements" ), pSubMenu );
            PopupMenu( &menu );
        }
        /// \todo disable read-only methods later?
        else if( ( prop_data->GetComponent().isMeth() ) /* && prop_data->GetComponent().isWriteable()*/ )
        {
            wxMenu menu( wxT( "" ) );
            menu.Append( miPopUpMethExec, wxT( "&Execute" ) );
            menu.Append( miPopUpDetailedFeatureInfo, wxT( "Detailed Feature Information" ) )->Enable( true );
            PopupMenu( &menu );
        }
    }
    OnPropertyRightClickedCustom( e );
}

//-----------------------------------------------------------------------------
void PropGridFrameBase::OnPropertySelected( wxPropertyGridEvent& e )
//-----------------------------------------------------------------------------
{
    if( !e.GetProperty() )
    {
        return;
    }

    OnPropertySelectedCustom( e );
}

//-----------------------------------------------------------------------------
void PropGridFrameBase::OnTimer( wxTimerEvent& e )
//-----------------------------------------------------------------------------
{
    try
    {
        switch( e.GetId() )
        {
        case teListUpdate:
            OnPropertyGridTimer();
            break;
        }
    }
    catch( const ImpactAcquireException& theException )
    {
        WriteLogMessage( wxString::Format( wxT( "%s: An exception was generated while updating the state of the property grid: %s(%s)\n" ), ConvertedString( __FUNCTION__ ).c_str(), ConvertedString( theException.getErrorString() ).c_str(), ConvertedString( theException.getErrorCodeAsString() ).c_str() ) );
    }
}

//-----------------------------------------------------------------------------
void PropGridFrameBase::SelectPropertyGrid( int id )
//-----------------------------------------------------------------------------
{
    PropGridMap::iterator it = m_propGrids.find( id );
    if( it != m_propGrids.end() )
    {
        m_pPGSelected = it->second;
        OnPropertyGridSelected();
    }
}

//-----------------------------------------------------------------------------
void PropGridFrameBase::SelectPropertyGrid( wxPropertyGrid* pGrid )
//-----------------------------------------------------------------------------
{
    PropGridMap::iterator it = m_propGrids.begin();
    const PropGridMap::iterator itEND = m_propGrids.end();
    while( it != itEND )
    {
        if( it->second == pGrid )
        {
            m_pPGSelected = pGrid;
            OnPropertyGridSelected();
            break;
        }
        ++it;
    }
}

//-----------------------------------------------------------------------------
bool PropGridFrameBase::SelectPropertyInPropertyGrid( PropData* pPropData )
//-----------------------------------------------------------------------------
{
    if( pPropData )
    {
        wxPGId id = pPropData->GetGridItem();
        if( wxPGIdIsOk( id ) )
        {
            wxPropertyGrid* pGrid = pPropData->GetParentGrid();
            if( pGrid && GetPropertyGrid() )
            {
                if( pGrid != GetPropertyGrid() )
                {
                    SelectPropertyGrid( pGrid );
                }
                if( wxPGIdToPtr( id )->GetParent() )
                {
                    ExpandPropertyRecursively( wxPGIdToPtr( id )->GetParent()->GetId() );
                }
                pGrid->Expand( id );
                return pGrid->SelectProperty( id, true );
            }
        }
    }
    return false;

}
//-----------------------------------------------------------------------------
void PropGridFrameBase::StartPropertyGridUpdateTimer( int period_ms )
//-----------------------------------------------------------------------------
{
    m_ListUpdateTimer.SetOwner( this, teListUpdate );
    m_ListUpdateTimer.Start( period_ms );
}

//-----------------------------------------------------------------------------
void PropGridFrameBase::StopPropertyGridUpdateTimer( void )
//-----------------------------------------------------------------------------
{
    if( m_ListUpdateTimer.IsRunning() )
    {
        m_ListUpdateTimer.Stop();
    }
}
