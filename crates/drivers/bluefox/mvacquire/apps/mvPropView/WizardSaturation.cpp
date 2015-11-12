#include <apps/Common/wxAbstraction.h>
#include "WizardSaturation.h"
#include <wx/slider.h>
#include <wx/tglbtn.h>

using namespace std;

//=============================================================================
//============== Implementation WizardColorCorrection =========================
//=============================================================================
BEGIN_EVENT_TABLE( WizardColorCorrection, OkAndCancelDlg )
    EVT_CHECKBOX( widCBEnableInputCorrectionMatrix, WizardColorCorrection::OnCBEnableInputCorrectionMatrix )
    EVT_TEXT( widInputCorrectionCombo, WizardColorCorrection::OnInputCorrectionComboTextChanged )
    EVT_CHECKBOX( widCBEnableOutputCorrectionMatrix, WizardColorCorrection::OnCBEnableOutputCorrectionMatrix )
    EVT_TEXT( widOutputCorrectionCombo, WizardColorCorrection::OnOutputCorrectionComboTextChanged )
    EVT_CHECKBOX( widCBSaturationEnable, WizardColorCorrection::OnCBSaturationEnable )
    EVT_COMMAND_SCROLL_THUMBTRACK( widSLSaturation, WizardColorCorrection::OnSLSaturation )
    EVT_SPINCTRL( widSCSaturation, WizardColorCorrection::OnSCSaturationChanged )
#ifdef BUILD_WITH_TEXT_EVENTS_FOR_SPINCTRL // BAT: Unfortunately on linux wxWidgets 2.6.x - ??? handling these messages will cause problems, while on Windows not doing so will not always update the GUI as desired :-(
    EVT_TEXT( widSCSaturation, WizardColorCorrection::OnSCSaturationTextChanged )
#endif // #ifdef BUILD_WITH_TEXT_EVENTS_FOR_SPINCTRL
    EVT_TOGGLEBUTTON( widTBSaturationDetails, WizardColorCorrection::OnBtnSaturationDetails )
    EVT_BUTTON( widBtnWriteToDevice, WizardColorCorrection::OnBtnWriteToDevice )
    EVT_BUTTON( widBtnWriteToDeviceAndSwitchOffHost, WizardColorCorrection::OnBtnWriteToDeviceAndSwitchOffHost )
    EVT_CHECKBOX( widCBEnableDeviceColorCorrection, WizardColorCorrection::OnCBEnableDeviceColorCorrection )
END_EVENT_TABLE()

//-----------------------------------------------------------------------------
WizardColorCorrection::WizardColorCorrection( wxWindow* pParent, const wxString& title, mvIMPACT::acquire::Device* pDev )
    : OkAndCancelDlg( pParent, widMainFrame, title, wxDefaultPosition, wxDefaultSize, wxDEFAULT_DIALOG_STYLE | wxRESIZE_BORDER | wxMAXIMIZE_BOX | wxMINIMIZE_BOX ),
      ip_( pDev ), pctc_( 0 ), pCBEnableInputCorrectionMatrix_( 0 ), pInputCorrectionCombo_( 0 ), pCBEnableOutputCorrectionMatrix_( 0 ),
      pOutputCorrectionCombo_( 0 ), pCBSaturationEnable_( 0 ), pSLSaturation_( 0 ), pSCSaturation_( 0 ), pTBSaturationDetails_( 0 ),
      pTCSaturationHints_( 0 ), pBtnWriteToDevice_( 0 ), pBtnWriteToDeviceAndSwitchOffHost_( 0 ), pCBEnableDeviceColorCorrection_( 0 ),
      pTopDownSizer_( 0 ), boGUICreated_( false )
//-----------------------------------------------------------------------------
{
    if( pDev->interfaceLayout.isValid() && ( pDev->interfaceLayout.read() == dilGenICam ) )
    {
        pctc_ = new mvIMPACT::acquire::GenICam::ColorTransformationControl( pDev );
        if( !pctc_->colorTransformationEnable.isValid() ||
            !pctc_->colorTransformationValue.isValid() ||
            !pctc_->colorTransformationValueSelector.isValid() )
        {
            delete pctc_;
            pctc_ = 0;
        }
    }
    CreateGUI();
}

//-----------------------------------------------------------------------------
void WizardColorCorrection::ApplySaturation( void )
//-----------------------------------------------------------------------------
{
    if( boGUICreated_ )
    {
        ApplySaturation( pCBSaturationEnable_->GetValue(), pSLSaturation_->GetValue() / 1000. );
    }
}

//-----------------------------------------------------------------------------
void WizardColorCorrection::ApplySaturation( const bool boEnable, const double K )
//-----------------------------------------------------------------------------
{
    try
    {
        ip_.colorTwistEnable.write( boEnable ? bTrue : bFalse );
        ip_.setSaturation( K );
    }
    catch( const ImpactAcquireException& e )
    {
        wxMessageBox( wxString::Format( wxT( "Failed to apply saturation(Error: %s(%s))" ), ConvertedString( e.getErrorString() ).c_str(), ConvertedString( e.getErrorCodeAsString() ).c_str() ), wxT( "Error" ), wxOK | wxICON_INFORMATION, this );
    }
}

//-----------------------------------------------------------------------------
void WizardColorCorrection::CreateGUI( void )
//-----------------------------------------------------------------------------
{
    /*
        |-------------------------------------|
        | pTopDownSizer                       |
        |                spacer               |
        | |---------------------------------| |
        | | slider  spin control            | |
        | |---------------------------------| |
        |                spacer               |
        | |---------------------------------| |
        | | saturation hints                | |
        | |---------------------------------| |
        |                spacer               |
        | |---------------------------------| |
        | | pButtonSizer                    | |
        | |---------------------------------| |
        |-------------------------------------|
    */

    wxPanel* pPanel = new wxPanel( this );

    pCBSaturationEnable_ = new wxCheckBox( pPanel, widCBSaturationEnable, wxT( "Enable" ) );
    pCBSaturationEnable_->SetToolTip( wxT( "Enables/disables the color twist matrix for saturation adjustment" ) );
    pSLSaturation_ = new wxSlider( pPanel, widSLSaturation, 1000, -10000, 10000, wxDefaultPosition, wxSize( 150, -1 ), wxSL_HORIZONTAL );

    wxBoxSizer* pSaturationControlsSizer = new wxBoxSizer( wxHORIZONTAL );
    pSaturationControlsSizer->Add( new wxStaticText( pPanel, wxID_ANY, wxT( " Saturation:" ) ) );
    pSaturationControlsSizer->Add( pSLSaturation_, wxSizerFlags().Expand() );
    pSCSaturation_ = new wxSpinCtrlDbl();
    pSCSaturation_->Create( pPanel, widSCSaturation, wxEmptyString, wxDefaultPosition, wxDefaultSize, wxSP_ARROW_KEYS, -10., 10., 1., 0.001, wxSPINCTRLDBL_AUTODIGITS, wxT( "%.3f" ) );
    pSCSaturation_->SetMode( mDouble );
    pSaturationControlsSizer->Add( pSCSaturation_, wxSizerFlags().Expand() );

    pTBSaturationDetails_ = new wxToggleButton( pPanel, widTBSaturationDetails, wxT( "&Details >>>" ) );
    wxFont fixedPitchFont( 10, wxFONTFAMILY_MODERN, wxFONTSTYLE_NORMAL, wxFONTWEIGHT_NORMAL );
    fixedPitchFont.SetWeight( wxFONTWEIGHT_BOLD );
    fixedPitchFont.SetUnderlined( true );
    wxTextAttr fixedPitchStyle;
    fixedPitchStyle.SetFont( fixedPitchFont );
    pTCSaturationHints_ = new wxTextCtrl( pPanel, wxID_ANY, wxEmptyString, wxDefaultPosition, wxDefaultSize, wxTE_MULTILINE | wxSUNKEN_BORDER | wxTE_RICH | wxTE_READONLY );
    WriteToTextCtrl( pTCSaturationHints_, wxT( "Saturation Formula:\n" ), fixedPitchStyle );
    fixedPitchFont.SetWeight( wxFONTWEIGHT_NORMAL );
    fixedPitchFont.SetUnderlined( false );
    fixedPitchStyle.SetFont( fixedPitchFont );
    WriteToTextCtrl( pTCSaturationHints_, wxT( " [0.299 + 0.701*K       , 0.587*(1-K) , 0.114*(1-K)     ]\n" ), fixedPitchStyle );
    WriteToTextCtrl( pTCSaturationHints_, wxT( " [0.299*(1-K) + 0.701*K , 0.587       , 0.114*(1-K)     ]\n" ), fixedPitchStyle );
    WriteToTextCtrl( pTCSaturationHints_, wxT( " [0.299*(1-K) + 0.701*K , 0.587*(1-K) , 0.114 + 0.886*K ]\n" ), fixedPitchStyle );
    WriteToTextCtrl( pTCSaturationHints_, wxT( "\n" ), fixedPitchStyle );
    WriteToTextCtrl( pTCSaturationHints_, wxT( "K is the saturation factor\n" ), fixedPitchStyle );
    WriteToTextCtrl( pTCSaturationHints_, wxT( "K > 1 increases saturation\n" ), fixedPitchStyle );
    WriteToTextCtrl( pTCSaturationHints_, wxT( "K = 1 means no change\n" ), fixedPitchStyle );
    WriteToTextCtrl( pTCSaturationHints_, wxT( "0 < K < 1 decreases saturation\n" ), fixedPitchStyle );
    WriteToTextCtrl( pTCSaturationHints_, wxT( "K = 0 produces B&W\n" ), fixedPitchStyle );
    WriteToTextCtrl( pTCSaturationHints_, wxT( "K < 0 inverts color\n" ), fixedPitchStyle );
    pTCSaturationHints_->ScrollLines( -( 256 * 256 ) ); // make sure the text control always shows the beginning of the help text
    pTCSaturationHints_->Hide();

    pCBEnableInputCorrectionMatrix_ = new wxCheckBox( pPanel, widCBEnableInputCorrectionMatrix, wxT( "Enable" ) );
    pCBEnableInputCorrectionMatrix_->SetToolTip( wxT( "Enables/disables the input color correction matrix" ) );
    wxArrayString inputCorrectionMatrixChoices;
    wxString inputCorrectionMatrixModeValue = ReadStringDict<PropertyIColorTwistInputCorrectionMatrixMode, TColorTwistInputCorrectionMatrixMode>( ip_.colorTwistInputCorrectionMatrixMode, inputCorrectionMatrixChoices );
    pInputCorrectionCombo_ = new wxComboBox( pPanel, widInputCorrectionCombo, inputCorrectionMatrixModeValue, wxDefaultPosition, wxSize( 200, wxDefaultCoord ), inputCorrectionMatrixChoices, wxCB_DROPDOWN | wxCB_READONLY );

    wxBoxSizer* pInputCorrectionSizer = new wxStaticBoxSizer( wxVERTICAL, pPanel, wxT( "Input Color Correction: " ) );
    pInputCorrectionSizer->AddSpacer( 5 );
    pInputCorrectionSizer->Add( pCBEnableInputCorrectionMatrix_ );
    pInputCorrectionSizer->AddSpacer( 10 );
    pInputCorrectionSizer->Add( pInputCorrectionCombo_ );
    pInputCorrectionSizer->AddSpacer( 5 );

    wxBoxSizer* pSaturationSizer = new wxStaticBoxSizer( wxVERTICAL, pPanel, wxT( "Saturation: " ) );
    pSaturationSizer->AddSpacer( 5 );
    pSaturationSizer->Add( pCBSaturationEnable_ );
    pSaturationSizer->AddSpacer( 10 );
    pSaturationSizer->Add( pSaturationControlsSizer );
    pSaturationSizer->AddSpacer( 10 );
    pSaturationSizer->Add( pTBSaturationDetails_ );
    pSaturationSizer->Add( pTCSaturationHints_, wxSizerFlags( 3 ).Expand() );
    pSaturationSizer->AddSpacer( 5 );

    pCBEnableOutputCorrectionMatrix_ = new wxCheckBox( pPanel, widCBEnableOutputCorrectionMatrix, wxT( "Enable" ) );
    pCBEnableOutputCorrectionMatrix_->SetToolTip( wxT( "Enables/disables the output color correction matrix" ) );
    wxArrayString outputCorrectionMatrixChoices;
    wxString outputCorrectionMatrixModeValue = ReadStringDict<PropertyIColorTwistOutputCorrectionMatrixMode, TColorTwistOutputCorrectionMatrixMode>( ip_.colorTwistOutputCorrectionMatrixMode, outputCorrectionMatrixChoices );
    pOutputCorrectionCombo_ = new wxComboBox( pPanel, widOutputCorrectionCombo, outputCorrectionMatrixModeValue, wxDefaultPosition, wxSize( 200, wxDefaultCoord ), outputCorrectionMatrixChoices, wxCB_DROPDOWN | wxCB_READONLY );

    wxBoxSizer* pOutputCorrectionSizer = new wxStaticBoxSizer( wxVERTICAL, pPanel, wxT( "Output Color Correction: " ) );
    pOutputCorrectionSizer->AddSpacer( 5 );
    pOutputCorrectionSizer->Add( pCBEnableOutputCorrectionMatrix_ );
    pOutputCorrectionSizer->AddSpacer( 10 );
    pOutputCorrectionSizer->Add( pOutputCorrectionCombo_ );
    pOutputCorrectionSizer->AddSpacer( 5 );

    pBtnWriteToDevice_ = new wxButton( pPanel, widBtnWriteToDevice, wxT( "Write To Device" ) );
    pBtnWriteToDevice_->SetToolTip( wxT( "Will upload all settings which are currently computed on the host system to the device" ) );
    pBtnWriteToDeviceAndSwitchOffHost_ = new wxButton( pPanel, widBtnWriteToDeviceAndSwitchOffHost, wxT( "Write To Device And Switch Off Host Processing" ) );
    pBtnWriteToDeviceAndSwitchOffHost_->SetToolTip( wxT( "Will upload all settings which are currently computed on the host system to the device and will switch off all host processing afterwards" ) );
    pCBEnableDeviceColorCorrection_ = new wxCheckBox( pPanel, widCBEnableDeviceColorCorrection, wxT( "Enable" ) );
    pCBEnableDeviceColorCorrection_->SetToolTip( wxT( "Enables/disables the color correction on the device" ) );

    wxBoxSizer* pDeviceColorCorrectionSizer = new wxStaticBoxSizer( wxHORIZONTAL, pPanel, wxT( "Device Color Correction Controls: " ) );
    pDeviceColorCorrectionSizer->Add( pBtnWriteToDevice_ );
    pDeviceColorCorrectionSizer->AddSpacer( 10 );
    pDeviceColorCorrectionSizer->Add( pBtnWriteToDeviceAndSwitchOffHost_ );
    pDeviceColorCorrectionSizer->AddSpacer( 10 );
    pDeviceColorCorrectionSizer->Add( pCBEnableDeviceColorCorrection_ );

    wxBoxSizer* pHostColorCorrectionSizer = new wxStaticBoxSizer( wxVERTICAL, pPanel, wxT( "Host Color Correction Controls: " ) );
    pHostColorCorrectionSizer->Add( pInputCorrectionSizer, wxSizerFlags().Expand() );
    pHostColorCorrectionSizer->Add( pSaturationSizer, wxSizerFlags( 8 ).Expand() );
    pHostColorCorrectionSizer->Add( pOutputCorrectionSizer, wxSizerFlags().Expand() );

    pTopDownSizer_ = new wxBoxSizer( wxVERTICAL );
    pTopDownSizer_->AddSpacer( 10 );
    pTopDownSizer_->Add( pHostColorCorrectionSizer, wxSizerFlags( 8 ).Expand() );
    pTopDownSizer_->AddSpacer( 10 );
    pTopDownSizer_->Add( pDeviceColorCorrectionSizer, wxSizerFlags().Expand() );
    pTopDownSizer_->AddSpacer( 10 );
    AddButtons( pPanel, pTopDownSizer_, true );
    FinalizeDlgCreation( pPanel, pTopDownSizer_ );
    ToggleDetails( false );

    boGUICreated_ = true;

    RefreshControls();
    SetupDlgControls();
}

//-----------------------------------------------------------------------------
void WizardColorCorrection::OnBtnWriteToDeviceAndSwitchOffHost( wxCommandEvent& )
//-----------------------------------------------------------------------------
{
    if( WriteToDevice() )
    {
        try
        {
            ip_.colorTwistInputCorrectionMatrixEnable.write( bFalse );
            ip_.colorTwistEnable.write( bFalse );
            ip_.colorTwistOutputCorrectionMatrixEnable.write( bFalse );
        }
        catch( const ImpactAcquireException& e )
        {
            wxMessageBox( wxString::Format( wxT( "Failed to modifiy driver properties(Error: %s)" ), ConvertedString( e.getErrorCodeAsString() ).c_str() ), wxT( "Failed To Write To Device" ), wxOK | wxICON_INFORMATION, this );
        }
        RefreshControls();
    }
}

//-----------------------------------------------------------------------------
void WizardColorCorrection::OnCBEnableDeviceColorCorrection( wxCommandEvent& e )
//-----------------------------------------------------------------------------
{
    if( !pctc_ )
    {
        return;
    }

    if( !pctc_->colorTransformationEnable.isWriteable() )
    {
        wxMessageBox( wxT( "The feature 'ColorTransformationEnable' on the device are currently not writeable. You might need to switch off streaming before the new settings can be applied." ), wxT( "Failed To Write To Device" ), wxOK | wxICON_INFORMATION, this );
        return;
    }

    try
    {
        pctc_->colorTransformationEnable.write( e.IsChecked() ? bTrue : bFalse );
    }
    catch( const ImpactAcquireException& e )
    {
        wxMessageBox( wxString::Format( wxT( "Failed To Write To Device(Error: %s)" ), ConvertedString( e.getErrorCodeAsString() ).c_str() ), wxT( "Failed To Write To Device" ), wxOK | wxICON_INFORMATION, this );
    }
}

//-----------------------------------------------------------------------------
void WizardColorCorrection::RefreshControls( void )
//-----------------------------------------------------------------------------
{
    const bool boInputCorrectionMatrixSupported = ip_.colorTwistInputCorrectionMatrixEnable.isValid();
    if( boInputCorrectionMatrixSupported )
    {
        pCBEnableInputCorrectionMatrix_->SetValue( ( ip_.colorTwistInputCorrectionMatrixEnable.read() == bTrue ) ? true : false );
        pInputCorrectionCombo_->SetValue( ConvertedString( ip_.colorTwistInputCorrectionMatrixMode.readS() ) );
    }
    pCBSaturationEnable_->SetValue( ( ip_.colorTwistEnable.read() == bTrue ) ? true : false );
    ApplySaturation();

    const bool boOutputCorrectionMatrixSupported = ip_.colorTwistOutputCorrectionMatrixEnable.isValid();
    if( boOutputCorrectionMatrixSupported )
    {
        pCBEnableOutputCorrectionMatrix_->SetValue( ( ip_.colorTwistOutputCorrectionMatrixEnable.read() == bTrue ) ? true : false );
        pOutputCorrectionCombo_->SetValue( ConvertedString( ip_.colorTwistOutputCorrectionMatrixMode.readS() ) );
    }

    if( pctc_ )
    {
        pCBEnableDeviceColorCorrection_->SetValue( ( pctc_->colorTransformationEnable.read() == bTrue ) ? true : false );
    }

    SetupDlgControls();
}

//-----------------------------------------------------------------------------
void WizardColorCorrection::SetupDlgControls( void )
//-----------------------------------------------------------------------------
{
    const bool boInputCorrectionMatrixSupported = ip_.colorTwistInputCorrectionMatrixEnable.isValid();
    pCBEnableInputCorrectionMatrix_->Enable( boInputCorrectionMatrixSupported );
    pInputCorrectionCombo_->Enable( boInputCorrectionMatrixSupported && pCBEnableInputCorrectionMatrix_->IsChecked() );

    pSLSaturation_->Enable( pCBSaturationEnable_->IsChecked() );
    pSCSaturation_->Enable( pCBSaturationEnable_->IsChecked() );

    const bool boOutputCorrectionMatrixSupported = ip_.colorTwistOutputCorrectionMatrixEnable.isValid();
    pCBEnableOutputCorrectionMatrix_->Enable( boOutputCorrectionMatrixSupported );
    pOutputCorrectionCombo_->Enable( boOutputCorrectionMatrixSupported && pCBEnableOutputCorrectionMatrix_->IsChecked() );

    const bool boDeviceColorCorrectionSupported = pctc_ != 0;
    pBtnWriteToDevice_->Enable( boDeviceColorCorrectionSupported );
    pBtnWriteToDeviceAndSwitchOffHost_->Enable( boDeviceColorCorrectionSupported );
    pCBEnableDeviceColorCorrection_->Enable( boDeviceColorCorrectionSupported );
}

//-----------------------------------------------------------------------------
void WizardColorCorrection::ToggleDetails( bool boShow )
//-----------------------------------------------------------------------------
{
    if( boShow )
    {
        pTCSaturationHints_->Show();
        SetSize( 560, 640 );
        pTBSaturationDetails_->SetLabel( wxT( "Less &Details <<<" ) );
    }
    else
    {
        pTCSaturationHints_->Hide();
        SetSize( 560, 340 );
        pTBSaturationDetails_->SetLabel( wxT( "&Details >>>" ) );
    }
    pTopDownSizer_->Layout();
}

//-----------------------------------------------------------------------------
bool WizardColorCorrection::WriteToDevice( void )
//-----------------------------------------------------------------------------
{
    if( !pctc_ )
    {
        return false;
    }

    if( !pctc_->colorTransformationValueSelector.isWriteable() ||
        !pctc_->colorTransformationValue.isWriteable() )
    {
        wxMessageBox( wxT( "One or more features color transformation control related features on the device are currently not writeable. You might need to switch off streaming before the new settings can be applied." ), wxT( "Failed To Write To Device" ), wxOK | wxICON_INFORMATION, this );
        return false;
    }

    try
    {
        std::vector<double> row;
        ip_.colorTwistResultingMatrixRow0.read( row, 0, 2 );
        pctc_->colorTransformationValueSelector.writeS( "Gain00" );
        pctc_->colorTransformationValue.write( row[0] );
        pctc_->colorTransformationValueSelector.writeS( "Gain01" );
        pctc_->colorTransformationValue.write( row[1] );
        pctc_->colorTransformationValueSelector.writeS( "Gain02" );
        pctc_->colorTransformationValue.write( row[2] );

        ip_.colorTwistResultingMatrixRow1.read( row, 0, 2 );
        pctc_->colorTransformationValueSelector.writeS( "Gain10" );
        pctc_->colorTransformationValue.write( row[0] );
        pctc_->colorTransformationValueSelector.writeS( "Gain11" );
        pctc_->colorTransformationValue.write( row[1] );
        pctc_->colorTransformationValueSelector.writeS( "Gain12" );
        pctc_->colorTransformationValue.write( row[2] );

        ip_.colorTwistResultingMatrixRow2.read( row, 0, 2 );
        pctc_->colorTransformationValueSelector.writeS( "Gain20" );
        pctc_->colorTransformationValue.write( row[0] );
        pctc_->colorTransformationValueSelector.writeS( "Gain21" );
        pctc_->colorTransformationValue.write( row[1] );
        pctc_->colorTransformationValueSelector.writeS( "Gain22" );
        pctc_->colorTransformationValue.write( row[2] );
    }
    catch( const ImpactAcquireException& e )
    {
        wxMessageBox( wxString::Format( wxT( "Failed To Write To Device(Error: %s)" ), ConvertedString( e.getErrorCodeAsString() ).c_str() ), wxT( "Failed To Write To Device" ), wxOK | wxICON_INFORMATION, this );
        return false;
    }
    return true;
}
